use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::common::{fence_marker_of, parse_heading};
use crate::server::application::chunking::{ChunkOutput, TextChunker};
use crate::server::application::ports::{ChatClient, ChatRequest, ChatResponseFormat};
use crate::server::application::AppError;
use crate::shared::{ChunkStrategy, ChunkingConfig, SettingsDto};

const SYSTEM_PROMPT: &str = "You are an assistant specialized in splitting text into \
    thematically consistent sections. The text has been divided into chunks, each marked with \
    <|start_chunk_X|> and <|end_chunk_X|> tags, where X is the chunk number. Your task is to \
    identify the points where splits should occur, such that consecutive chunks of similar themes \
    stay together. Respond with a list of chunk IDs where you believe a split should be made. \
    For example, if chunks 1 and 2 belong together but chunk 3 starts a new topic, you would \
    suggest a split after chunk 2. THE CHUNK IDs MUST BE IN ASCENDING ORDER. \
    Your response should be in the form: 'split_after: 3, 5'.";

const WINDOW_CHARS: usize = 3000;

#[derive(Debug, Clone)]
struct MicroChunk {
    text: String,
    char_start: usize,
    char_end: usize,
}

#[derive(Debug, Clone)]
struct TextUnit {
    text: String,
    char_start: usize,
    char_end: usize,
    atomic: bool,
}

#[derive(Debug, Clone, Copy)]
struct SourceLine<'a> {
    raw: &'a str,
    line: &'a str,
    char_start: usize,
    char_end: usize,
}

pub struct LlmChunker {
    chat_client: Arc<dyn ChatClient>,
    settings: Arc<RwLock<SettingsDto>>,
}

impl LlmChunker {
    pub fn create(chat_client: Arc<dyn ChatClient>, settings: Arc<RwLock<SettingsDto>>) -> Self {
        Self {
            chat_client,
            settings,
        }
    }
}

#[async_trait]
impl TextChunker for LlmChunker {
    fn strategy(&self) -> ChunkStrategy {
        ChunkStrategy::Llm
    }

    async fn chunk(
        &self,
        config: ChunkingConfig,
        source: &str,
    ) -> Result<Vec<ChunkOutput>, AppError> {
        let micro_size = config.llm_micro_chunk_chars.max(100) as usize;
        let micro_chunks = split_into_micro_chunks(source, micro_size);

        if micro_chunks.is_empty() {
            return Ok(Vec::new());
        }

        let model = self
            .settings
            .read()
            .await
            .evaluation
            .generation_model
            .clone();
        let split_points =
            find_split_points(&micro_chunks, self.chat_client.as_ref(), &model).await?;
        let merged = merge_micro_chunks(&micro_chunks, &split_points);

        Ok(merged
            .into_iter()
            .enumerate()
            .map(|(i, mc)| chunk_output_from_micro_chunk(i, mc))
            .collect())
    }
}

fn chunk_output_from_micro_chunk(chunk_id: usize, mc: MicroChunk) -> ChunkOutput {
    let (text, char_start, char_end) = trim_chunk_text(&mc.text, mc.char_start, mc.char_end);
    let heading = extract_heading(&text);
    ChunkOutput {
        chunk_id: chunk_id as u32,
        heading,
        text,
        char_start: char_start as u32,
        char_end: char_end as u32,
    }
}

fn trim_chunk_text(text: &str, char_start: usize, char_end: usize) -> (String, usize, usize) {
    let text_len = text.chars().count();
    let leading = text.chars().take_while(|c| c.is_whitespace()).count();
    let trailing = text.chars().rev().take_while(|c| c.is_whitespace()).count();

    if leading + trailing >= text_len {
        return (String::new(), char_start, char_start);
    }

    let trimmed: String = text
        .chars()
        .skip(leading)
        .take(text_len - leading - trailing)
        .collect();

    (
        trimmed,
        char_start + leading,
        char_end.saturating_sub(trailing),
    )
}

fn split_into_micro_chunks(body: &str, target_chars: usize) -> Vec<MicroChunk> {
    let target_chars = target_chars.max(1);
    pack_text_units(split_into_text_units(body), target_chars)
}

fn split_into_text_units(body: &str) -> Vec<TextUnit> {
    let mut units = Vec::new();
    let lines = source_lines(body);
    let mut idx = 0usize;
    let mut prose = String::new();
    let mut prose_start: Option<usize> = None;

    while idx < lines.len() {
        let line = lines[idx];

        if let Some(marker) = fence_marker_of(line.line) {
            flush_prose(&mut units, &mut prose, &mut prose_start);
            let fence = collect_fenced_block(&lines, &mut idx, &marker);
            push_unit(
                &mut units,
                fence.text,
                fence.char_start,
                fence.char_end,
                true,
            );
            continue;
        }

        if parse_heading(line.line).is_some() {
            flush_prose(&mut units, &mut prose, &mut prose_start);
            push_unit(
                &mut units,
                line.raw.to_string(),
                line.char_start,
                line.char_end,
                true,
            );
            continue;
        }

        if is_list_item(line.line) {
            let list = collect_list_block(&lines, &mut idx);
            if list_intro_should_bind(&prose) {
                let start = prose_start.take().unwrap_or(list.char_start);
                let mut text = std::mem::take(&mut prose);
                text.push_str(&list.text);
                push_unit(&mut units, text, start, list.char_end, true);
            } else {
                flush_prose(&mut units, &mut prose, &mut prose_start);
                push_unit(&mut units, list.text, list.char_start, list.char_end, true);
            }
            continue;
        }

        if prose_start.is_none() {
            prose_start = Some(line.char_start);
        }
        prose.push_str(line.raw);
        idx += 1;
    }

    flush_prose(&mut units, &mut prose, &mut prose_start);

    units
}

fn source_lines(body: &str) -> Vec<SourceLine<'_>> {
    let mut lines = Vec::new();
    let mut char_cursor = 0usize;

    for raw in body.split_inclusive('\n') {
        let char_start = char_cursor;
        let char_end = char_start + raw.chars().count();
        lines.push(SourceLine {
            raw,
            line: raw.trim_end_matches('\n'),
            char_start,
            char_end,
        });
        char_cursor = char_end;
    }

    if body.is_empty() {
        return lines;
    }

    if !body.ends_with('\n') && lines.is_empty() {
        lines.push(SourceLine {
            raw: body,
            line: body,
            char_start: 0,
            char_end: body.chars().count(),
        });
    }

    lines
}

fn collect_fenced_block(lines: &[SourceLine<'_>], idx: &mut usize, marker: &str) -> TextUnit {
    let start = lines[*idx].char_start;
    let mut end = lines[*idx].char_end;
    let mut text = String::new();
    let mut first = true;

    while *idx < lines.len() {
        let line = lines[*idx];
        text.push_str(line.raw);
        end = line.char_end;
        *idx += 1;

        if !first && fence_marker_of(line.line).is_some() && line.line.starts_with(marker) {
            break;
        }
        first = false;
    }

    TextUnit {
        text,
        char_start: start,
        char_end: end,
        atomic: true,
    }
}

fn collect_list_block(lines: &[SourceLine<'_>], idx: &mut usize) -> TextUnit {
    let start = lines[*idx].char_start;
    let mut end = lines[*idx].char_end;
    let mut text = String::new();

    while *idx < lines.len() {
        let line = lines[*idx];
        if text.is_empty()
            || is_list_item(line.line)
            || is_list_continuation(line.line)
            || blank_line_inside_list(lines, *idx)
        {
            text.push_str(line.raw);
            end = line.char_end;
            *idx += 1;
            continue;
        }
        break;
    }

    if *idx < lines.len() && lines[*idx].line.trim().is_empty() {
        let line = lines[*idx];
        text.push_str(line.raw);
        end = line.char_end;
        *idx += 1;
    }

    TextUnit {
        text,
        char_start: start,
        char_end: end,
        atomic: true,
    }
}

fn blank_line_inside_list(lines: &[SourceLine<'_>], idx: usize) -> bool {
    lines[idx].line.trim().is_empty()
        && lines
            .iter()
            .skip(idx + 1)
            .find(|line| !line.line.trim().is_empty())
            .map(|line| is_list_item(line.line) || is_list_continuation(line.line))
            .unwrap_or(false)
}

fn list_intro_should_bind(prose: &str) -> bool {
    let trimmed = prose.trim_end();
    !trimmed.is_empty() && trimmed.ends_with(':')
}

fn flush_prose(units: &mut Vec<TextUnit>, prose: &mut String, prose_start: &mut Option<usize>) {
    let Some(start) = prose_start.take() else {
        return;
    };
    let text = std::mem::take(prose);
    units.extend(split_prose_on_terminators(&text, start));
}

fn split_prose_on_terminators(text: &str, char_start: usize) -> Vec<TextUnit> {
    let chars: Vec<char> = text.chars().collect();
    let mut units = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;

    while i < chars.len() {
        if is_terminating_punctuation(chars[i])
            && chars.get(i + 1).map(|c| c.is_whitespace()).unwrap_or(true)
        {
            let mut end = i + 1;
            while chars.get(end).map(|c| c.is_whitespace()).unwrap_or(false) {
                end += 1;
            }
            push_unit_from_chars(&mut units, &chars, start, end, char_start, false);
            start = end;
            i = end;
            continue;
        }
        i += 1;
    }

    if start < chars.len() {
        push_unit_from_chars(&mut units, &chars, start, chars.len(), char_start, false);
    }

    units
}

fn push_unit_from_chars(
    units: &mut Vec<TextUnit>,
    chars: &[char],
    start: usize,
    end: usize,
    base_char_start: usize,
    atomic: bool,
) {
    let text: String = chars[start..end].iter().collect();
    push_unit(
        units,
        text,
        base_char_start + start,
        base_char_start + end,
        atomic,
    );
}

fn push_unit(
    units: &mut Vec<TextUnit>,
    text: String,
    char_start: usize,
    char_end: usize,
    atomic: bool,
) {
    if text.trim().is_empty() {
        return;
    }
    units.push(TextUnit {
        text,
        char_start,
        char_end,
        atomic,
    });
}

fn pack_text_units(units: Vec<TextUnit>, target_chars: usize) -> Vec<MicroChunk> {
    let mut out = Vec::new();
    let mut current_text = String::new();
    let mut current_start = 0usize;
    let mut current_end = 0usize;

    for unit in units {
        if unit.atomic {
            flush_micro_chunk(&mut out, &mut current_text, current_start, current_end);
            out.push(MicroChunk {
                text: unit.text,
                char_start: unit.char_start,
                char_end: unit.char_end,
            });
            continue;
        }

        let pieces = split_oversized_unit(unit, target_chars);
        for piece in pieces {
            let current_len = current_text.chars().count();
            let piece_len = piece.text.chars().count();
            if !current_text.is_empty() && current_len + piece_len > target_chars {
                flush_micro_chunk(&mut out, &mut current_text, current_start, current_end);
            }

            if current_text.is_empty() {
                current_start = piece.char_start;
            }
            current_end = piece.char_end;
            current_text.push_str(&piece.text);
        }
    }

    flush_micro_chunk(&mut out, &mut current_text, current_start, current_end);
    out
}

fn flush_micro_chunk(
    out: &mut Vec<MicroChunk>,
    current_text: &mut String,
    current_start: usize,
    current_end: usize,
) {
    if current_text.trim().is_empty() {
        current_text.clear();
        return;
    }
    out.push(MicroChunk {
        text: std::mem::take(current_text),
        char_start: current_start,
        char_end: current_end,
    });
}

fn split_oversized_unit(unit: TextUnit, target_chars: usize) -> Vec<TextUnit> {
    if unit.atomic || unit.text.chars().count() <= target_chars * 2 {
        return vec![unit];
    }

    let chars: Vec<char> = unit.text.chars().collect();
    let mut out = Vec::new();
    let mut start = 0usize;

    while start < chars.len() {
        let hard_end = (start + target_chars).min(chars.len());
        let end = if hard_end < chars.len() {
            last_whitespace_after_half(&chars[start..hard_end])
                .map(|offset| start + offset + 1)
                .unwrap_or(hard_end)
        } else {
            hard_end
        };
        push_unit_from_chars(&mut out, &chars, start, end, unit.char_start, false);
        start = end;
    }

    out
}

fn last_whitespace_after_half(chars: &[char]) -> Option<usize> {
    let half = chars.len() / 2;
    chars
        .iter()
        .enumerate()
        .rev()
        .find(|(i, c)| *i >= half && c.is_whitespace())
        .map(|(i, _)| i)
}

fn is_terminating_punctuation(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | ';' | ':' | '。' | '！' | '？')
}

fn is_list_item(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
        || trimmed.starts_with("+ ")
        || ordered_list_marker(trimmed)
}

fn is_list_continuation(line: &str) -> bool {
    if line.trim().is_empty() || parse_heading(line).is_some() || fence_marker_of(line).is_some() {
        return false;
    }
    line.starts_with("  ") || line.starts_with('\t')
}

fn ordered_list_marker(line: &str) -> bool {
    let mut chars = line.chars().peekable();
    let mut saw_digit = false;
    while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
        saw_digit = true;
        chars.next();
    }
    saw_digit && matches!(chars.next(), Some('.' | ')')) && matches!(chars.next(), Some(' '))
}

async fn find_split_points(
    micro_chunks: &[MicroChunk],
    client: &dyn ChatClient,
    model: &str,
) -> Result<Vec<usize>, AppError> {
    let n = micro_chunks.len();
    if n <= 1 {
        return Ok(vec![n.saturating_sub(1)]);
    }

    let mut split_after: Vec<usize> = Vec::new();
    let mut current = 0usize;

    while current < n.saturating_sub(3) {
        let mut window = String::new();

        for (i, item) in micro_chunks.iter().enumerate().take(n).skip(current) {
            window.push_str(&format!(
                "<|start_chunk_{}|>{}<|end_chunk_{}|>",
                i + 1,
                item.text,
                i + 1
            ));
            if window.len() > WINDOW_CHARS {
                break;
            }
        }

        let user_msg = format!(
            "CHUNKED_TEXT: {window}\n\nRespond only with the IDs of chunks where a split should occur. \
            You MUST respond with at least one split. IDs must be >= {}.",
            current + 1
        );

        let response = client
            .chat(ChatRequest {
                model: model.to_string(),
                system: SYSTEM_PROMPT.to_string(),
                user: user_msg,
                temperature: 0.5,
                response_format: ChatResponseFormat::Text,
            })
            .await?;
        let numbers = parse_split_response(&response.content, current + 1);

        if numbers.is_empty() {
            // No valid split found; advance one step to avoid infinite loop
            current += 1;
            continue;
        }

        for &n_id in &numbers {
            let idx = n_id - 1; // convert 1-indexed to 0-indexed
            if idx < n && !split_after.contains(&idx) {
                split_after.push(idx);
            }
        }

        current = *numbers.last().unwrap();
    }

    // Ensure the last chunk is always included
    if split_after.last() != Some(&(n - 1)) {
        split_after.push(n - 1);
    }

    split_after.sort_unstable();
    split_after.dedup();
    Ok(split_after)
}

fn parse_split_response(response: &str, min_id: usize) -> Vec<usize> {
    let line = response
        .lines()
        .find(|l| l.contains("split_after:"))
        .unwrap_or(response);

    let numbers: Vec<usize> = line
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|s| s.parse::<usize>().ok())
        .filter(|&n| n >= min_id)
        .collect();

    // Verify ascending order
    let is_ascending = numbers.windows(2).all(|w| w[0] < w[1]);
    if !is_ascending {
        return Vec::new();
    }

    numbers
}

fn merge_micro_chunks(micro_chunks: &[MicroChunk], split_points: &[usize]) -> Vec<MicroChunk> {
    let mut out = Vec::new();
    let mut current_text = String::new();
    let mut current_start = micro_chunks.first().map(|m| m.char_start).unwrap_or(0);
    let mut last_end = 0usize;

    for (i, mc) in micro_chunks.iter().enumerate() {
        current_text.push_str(&mc.text);
        last_end = mc.char_end;

        if split_points.contains(&i) {
            out.push(MicroChunk {
                text: current_text.clone(),
                char_start: current_start,
                char_end: last_end,
            });
            current_text.clear();
            if i + 1 < micro_chunks.len() {
                current_start = micro_chunks[i + 1].char_start;
            }
        }
    }

    if !current_text.trim().is_empty() {
        out.push(MicroChunk {
            text: current_text,
            char_start: current_start,
            char_end: last_end,
        });
    }

    out
}

fn extract_heading(text: &str) -> String {
    for line in text.lines() {
        if let Some((_depth, heading)) = parse_heading(line) {
            return heading;
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joined_text(chunks: &[MicroChunk]) -> String {
        chunks.iter().map(|c| c.text.as_str()).collect()
    }

    fn slice_chars(text: &str, start: usize, end: usize) -> String {
        text.chars().skip(start).take(end - start).collect()
    }

    #[test]
    fn split_into_micro_chunks_basic() {
        let body = "paragraph one.\n\nparagraph two.\n\nparagraph three.";
        let chunks = split_into_micro_chunks(body, 10);
        assert!(chunks.len() >= 2);
        let full = joined_text(&chunks);
        assert!(full.contains("paragraph one"));
        assert!(full.contains("paragraph three"));
    }

    #[test]
    fn split_into_micro_chunks_uses_terminating_punctuation_boundaries() {
        let body = "Alpha sentence. Beta clause; Gamma question? Delta exclaim!";
        let chunks = split_into_micro_chunks(body, 25);

        assert_eq!(joined_text(&chunks), body);
        assert_eq!(
            chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(),
            vec![
                "Alpha sentence. ",
                "Beta clause; ",
                "Gamma question? ",
                "Delta exclaim!"
            ]
        );
        for chunk in chunks {
            assert_eq!(
                slice_chars(body, chunk.char_start, chunk.char_end),
                chunk.text
            );
        }
    }

    #[test]
    fn split_into_micro_chunks_keeps_headings_atomic() {
        let body = "Intro sentence.\n## Heading\nBody sentence.";
        let chunks = split_into_micro_chunks(body, 1000);
        let heading_start = "Intro sentence.\n".chars().count();

        assert_eq!(joined_text(&chunks), body);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[1].text, "## Heading\n");
        assert_eq!(chunks[1].char_start, heading_start);
        assert_eq!(
            slice_chars(body, chunks[1].char_start, chunks[1].char_end),
            chunks[1].text
        );
    }

    #[test]
    fn split_into_micro_chunks_keeps_fenced_code_blocks_atomic() {
        let fence = "```rust\nfn main() { println!(\"a.b\"); }\n```\n";
        let body = format!("Intro sentence.\n{fence}After.");
        let chunks = split_into_micro_chunks(&body, 10);

        assert_eq!(joined_text(&chunks), body);
        let fence_chunks = chunks
            .iter()
            .filter(|chunk| chunk.text.contains("```rust"))
            .collect::<Vec<_>>();
        assert_eq!(fence_chunks.len(), 1);
        assert_eq!(fence_chunks[0].text, fence);
    }

    #[test]
    fn split_into_micro_chunks_keeps_contiguous_list_items_together() {
        let body = "- first item.\n- second item.\nclosing sentence.";
        let chunks = split_into_micro_chunks(body, 1000);

        assert_eq!(joined_text(&chunks), body);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text, "- first item.\n- second item.\n");
        assert_eq!(chunks[1].text, "closing sentence.");
    }

    #[test]
    fn split_into_micro_chunks_binds_colon_intro_to_numbered_list() {
        let body = concat!(
            "Durable Objects give you:\n\n",
            "1. A single-threaded execution context per aggregate ID.\n",
            "2. In-process, synchronous SQLite. Projections catch up in milliseconds.\n",
            "3. Alarms, as the do-something-about-events mechanism.\n\n",
            "There are catches:\n\n",
            "- Storage per DO is capped.\n",
            "- Cross-aggregate queries need D1.\n"
        );
        let chunks = split_into_micro_chunks(body, 150);

        assert_eq!(joined_text(&chunks), body);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].text.starts_with("Durable Objects give you:"));
        assert!(chunks[0].text.contains("1. A single-threaded"));
        assert!(chunks[0].text.contains("3. Alarms"));
        assert!(chunks[1].text.starts_with("There are catches:"));
        assert!(chunks[1].text.contains("- Storage per DO"));
        for chunk in chunks {
            assert_eq!(
                slice_chars(body, chunk.char_start, chunk.char_end),
                chunk.text
            );
        }
    }

    #[test]
    fn split_into_micro_chunks_keeps_wrapped_list_item_with_item() {
        let body = "- first item starts here\n  and continues here.\n- second item.\n";
        let chunks = split_into_micro_chunks(body, 1000);

        assert_eq!(joined_text(&chunks), body);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, body);
    }

    #[test]
    fn split_into_micro_chunks_splits_oversized_prose_without_punctuation() {
        let body = "alpha beta gamma delta epsilon zeta eta theta";
        let chunks = split_into_micro_chunks(body, 12);

        assert!(chunks.len() > 1);
        assert_eq!(joined_text(&chunks), body);
        assert!(chunks.iter().all(|chunk| !chunk.text.trim().is_empty()));
    }

    #[test]
    fn chunk_output_trims_text_and_preserves_offsets() {
        let text = "\n  £ Trimmed text. \n";
        let mc = MicroChunk {
            text: text.to_string(),
            char_start: 7,
            char_end: 7 + text.chars().count(),
        };

        let output = chunk_output_from_micro_chunk(2, mc);

        assert_eq!(output.chunk_id, 2);
        assert_eq!(output.text, "£ Trimmed text.");
        assert_eq!(output.char_start, 10);
        assert_eq!(
            output.char_end,
            10 + "£ Trimmed text.".chars().count() as u32
        );
    }

    #[test]
    fn parse_split_response_basic() {
        let nums = parse_split_response("split_after: 2, 5, 8", 1);
        assert_eq!(nums, vec![2, 5, 8]);
    }

    #[test]
    fn parse_split_response_filters_below_min() {
        let nums = parse_split_response("split_after: 1, 2, 5", 3);
        assert_eq!(nums, vec![5]);
    }

    #[test]
    fn parse_split_response_rejects_non_ascending() {
        let nums = parse_split_response("split_after: 5, 3, 8", 1);
        assert!(nums.is_empty());
    }

    #[test]
    fn merge_micro_chunks_combines_correctly() {
        let micro = vec![
            MicroChunk {
                text: "A".into(),
                char_start: 0,
                char_end: 1,
            },
            MicroChunk {
                text: "B".into(),
                char_start: 1,
                char_end: 2,
            },
            MicroChunk {
                text: "C".into(),
                char_start: 2,
                char_end: 3,
            },
        ];
        // Split after index 1 (keep 0+1 together, 2 alone)
        let merged = merge_micro_chunks(&micro, &[1, 2]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].text, "AB");
        assert_eq!(merged[1].text, "C");
        assert_eq!(merged[0].char_start, 0);
        assert_eq!(merged[0].char_end, 2);
    }

    #[test]
    fn extract_heading_finds_first_heading() {
        let text = "some intro\n\n## My Section\n\nbody text";
        assert_eq!(extract_heading(text), "My Section");
    }

    #[test]
    fn extract_heading_returns_empty_when_none() {
        let text = "just plain text with no heading";
        assert_eq!(extract_heading(text), "");
    }
}
