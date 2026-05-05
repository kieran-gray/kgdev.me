use async_trait::async_trait;

use super::common::{fence_marker_of, parse_heading, split_into_lines};
use crate::server::application::chunking::{ChunkOutput, TextChunker};
use crate::server::application::AppError;
use crate::shared::{ChunkStrategy, ChunkingConfig};

#[derive(Debug, Clone)]
struct Segment {
    text: String,
    char_start: usize,
    char_end: usize,
    heading: String,
    atomic: bool,
}

pub struct BertChunker;

#[async_trait]
impl TextChunker for BertChunker {
    fn strategy(&self) -> ChunkStrategy {
        ChunkStrategy::Bert
    }

    async fn chunk(
        &self,
        config: ChunkingConfig,
        source: &str,
    ) -> Result<Vec<ChunkOutput>, AppError> {
        let target = config.target_chars.max(1) as usize;
        let overlap = config.overlap_chars as usize;
        let min = config.min_chars as usize;

        let body_chars: Vec<char> = source.chars().collect();
        let raw = parse_segments(&body_chars);
        let packed = pack_segments(raw, target);
        let split = split_oversized(packed, target, overlap, min);

        Ok(split
            .into_iter()
            .filter(|s| !s.text.trim().is_empty())
            .enumerate()
            .map(|(i, s)| ChunkOutput {
                chunk_id: i as u32,
                heading: s.heading,
                text: s.text.trim().to_string(),
                char_start: s.char_start as u32,
                char_end: s.char_end as u32,
            })
            .collect())
    }
}

fn parse_segments(body: &[char]) -> Vec<Segment> {
    let lines = split_into_lines(body);
    let mut segments: Vec<Segment> = Vec::new();
    let mut cursor: usize = 0;
    let mut heading_path: Vec<String> = Vec::new();

    let mut buf: Vec<String> = Vec::new();
    let mut buf_start: usize = 0;
    let mut buf_atomic = false;

    let mut in_fence = false;
    let mut fence_marker = String::new();

    let flush = |buf: &mut Vec<String>,
                 buf_start: &mut usize,
                 buf_atomic: &mut bool,
                 heading_path: &[String],
                 end_offset: usize,
                 segments: &mut Vec<Segment>| {
        if buf.is_empty() {
            return;
        }
        segments.push(Segment {
            text: buf.join("\n"),
            char_start: *buf_start,
            char_end: end_offset,
            heading: heading_path.join(" > "),
            atomic: *buf_atomic,
        });
        buf.clear();
        *buf_atomic = false;
    };

    for line in lines.iter() {
        let line_start = cursor;
        let line_len = line.chars().count();
        let line_end = cursor + line_len;
        let line_with_newline = line_end + 1;

        let fence_match = fence_marker_of(line);

        match (in_fence, fence_match) {
            (false, Some(fence_match)) => {
                flush(
                    &mut buf,
                    &mut buf_start,
                    &mut buf_atomic,
                    &heading_path,
                    line_start,
                    &mut segments,
                );
                in_fence = true;
                fence_marker = fence_match;
                buf_start = line_start;
                buf_atomic = true;
                buf.push(line.clone());
                cursor = line_with_newline;
                continue;
            }
            (true, Some(_fence_match)) => {
                buf.push(line.clone());
                if line.starts_with(&fence_marker) {
                    in_fence = false;
                    fence_marker.clear();
                    flush(
                        &mut buf,
                        &mut buf_start,
                        &mut buf_atomic,
                        &heading_path,
                        line_with_newline,
                        &mut segments,
                    );
                }
                cursor = line_with_newline;
                continue;
            }
            _ => {}
        }

        if let Some((depth, text)) = parse_heading(line) {
            flush(
                &mut buf,
                &mut buf_start,
                &mut buf_atomic,
                &heading_path,
                line_start,
                &mut segments,
            );
            heading_path.truncate(depth - 1);
            while heading_path.len() < depth {
                heading_path.push(String::new());
            }
            heading_path[depth - 1] = text;
            buf_start = line_start;
            buf.push(line.clone());
            cursor = line_with_newline;
            continue;
        }

        if buf.is_empty() {
            buf_start = line_start;
        }
        buf.push(line.clone());
        cursor = line_with_newline;
    }

    flush(
        &mut buf,
        &mut buf_start,
        &mut buf_atomic,
        &heading_path,
        cursor,
        &mut segments,
    );

    segments
}

fn pack_segments(segments: Vec<Segment>, target: usize) -> Vec<Segment> {
    let mut packed: Vec<Segment> = Vec::new();
    let mut current: Option<Segment> = None;

    for seg in segments {
        let cur = match current.take() {
            None => {
                current = Some(seg);
                continue;
            }
            Some(c) => c,
        };

        if cur.atomic || seg.atomic || cur.heading != seg.heading {
            packed.push(cur);
            current = Some(seg);
            continue;
        }

        let merged_text = format!("{}\n{}", cur.text, seg.text);
        if merged_text.chars().count() <= target {
            current = Some(Segment {
                text: merged_text,
                char_start: cur.char_start,
                char_end: seg.char_end,
                heading: cur.heading,
                atomic: cur.atomic,
            });
            continue;
        }

        packed.push(cur);
        current = Some(seg);
    }

    if let Some(c) = current {
        packed.push(c);
    }
    packed
}

fn last_double_newline(window: &[char]) -> Option<usize> {
    if window.len() < 2 {
        return None;
    }
    let mut i = window.len() - 2;
    loop {
        if window[i] == '\n' && window[i + 1] == '\n' {
            return Some(i);
        }
        if i == 0 {
            return None;
        }
        i -= 1;
    }
}

/// Returns the index of the last sentence-ending punctuation in `window` if
/// and only if it is followed by whitespace (so the lookahead `[^.!?]*$` is satisfied).
fn last_sentence_break(window: &[char]) -> Option<usize> {
    let mut last_punct: Option<usize> = None;
    for (i, &c) in window.iter().enumerate() {
        if c == '.' || c == '!' || c == '?' {
            last_punct = Some(i);
        }
    }
    let i = last_punct?;
    if i + 1 < window.len() && window[i + 1].is_whitespace() {
        Some(i)
    } else {
        None
    }
}

fn split_oversized(
    segments: Vec<Segment>,
    target: usize,
    overlap: usize,
    min: usize,
) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    for seg in segments {
        let text_chars: Vec<char> = seg.text.chars().collect();
        if seg.atomic || text_chars.len() <= target + overlap {
            out.push(seg);
            continue;
        }

        let mut start = 0usize;
        let total = text_chars.len();
        loop {
            if start >= total {
                break;
            }
            let end = (start + target).min(total);
            let mut break_at = end;
            if end < total {
                let window = &text_chars[start..end];
                let last_para = last_double_newline(window);
                let last_sent = last_sentence_break(window);
                if let Some(p) = last_para {
                    if p > target / 2 {
                        break_at = start + p;
                    } else if let Some(s) = last_sent {
                        if s > target / 2 {
                            break_at = start + s + 1;
                        }
                    }
                } else if let Some(s) = last_sent {
                    if s > target / 2 {
                        break_at = start + s + 1;
                    }
                }
            }
            let raw_piece: String = text_chars[start..break_at].iter().collect();
            let piece = raw_piece.trim().to_string();
            let piece_len = piece.chars().count();
            if !piece.is_empty() {
                let last_atomic = out.last().map(|s| s.atomic).unwrap_or(true);
                if piece_len < min && !out.is_empty() && !last_atomic {
                    let prev = out.last_mut().unwrap();
                    prev.text = format!("{}\n\n{}", prev.text, piece);
                    prev.char_end = seg.char_start + break_at;
                } else {
                    out.push(Segment {
                        text: piece,
                        char_start: seg.char_start + start,
                        char_end: seg.char_start + break_at,
                        heading: seg.heading.clone(),
                        atomic: false,
                    });
                }
            }
            if break_at >= total {
                break;
            }
            let candidate = break_at.saturating_sub(overlap);
            start = if candidate > start {
                candidate
            } else {
                break_at
            };
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ChunkStrategy;

    fn cfg() -> ChunkingConfig {
        ChunkingConfig {
            strategy: ChunkStrategy::Bert,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn chunk_yields_sequential_ids() {
        let chunker = BertChunker {};

        let src = "## A\nfirst paragraph.\n\n## B\nsecond paragraph.";
        let chunks = chunker.chunk(cfg(), src).await.unwrap();
        assert!(!chunks.is_empty());
        for (i, c) in chunks.iter().enumerate() {
            assert_eq!(c.chunk_id as usize, i);
        }
    }

    #[tokio::test]
    async fn chunk_preserves_heading_path() {
        let chunker = BertChunker {};

        let src = "# Top\n\nintro\n\n## Sub\n\ndetail";
        let chunks = chunker.chunk(cfg(), src).await.unwrap();
        let headings: Vec<&str> = chunks.iter().map(|c| c.heading.as_str()).collect();
        assert!(headings.iter().any(|h| h.contains("Top")));
        assert!(headings.iter().any(|h| h.contains("Sub")));
    }

    #[tokio::test]
    async fn chunk_does_not_split_inside_fence() {
        let chunker = BertChunker {};

        let code: String = (0..200)
            .map(|i| format!("let x{i} = {i};"))
            .collect::<Vec<_>>()
            .join("\n");
        let src = format!("## Code\n\n```rust\n{code}\n```\n");
        let chunks = chunker.chunk(cfg(), &src).await.unwrap();
        for c in chunks.iter().filter(|c| c.text.contains("```")) {
            let opens = c.text.matches("```").count();
            assert_eq!(opens % 2, 0, "chunk {} has unbalanced fences", c.chunk_id);
        }
    }

    #[tokio::test]
    async fn smaller_target_yields_more_chunks() {
        let chunker = BertChunker {};

        let para = "Lorem ipsum dolor sit amet. ".repeat(120);
        let src = format!("## Big\n\n{para}");
        let big = ChunkingConfig {
            strategy: ChunkStrategy::Bert,
            target_chars: 1600,
            overlap_chars: 240,
            min_chars: 320,
            ..Default::default()
        };
        let small = ChunkingConfig {
            strategy: ChunkStrategy::Bert,
            target_chars: 600,
            overlap_chars: 80,
            min_chars: 120,
            ..Default::default()
        };
        let big_chunks = chunker.chunk(big, &src).await.unwrap();
        let small_chunks = chunker.chunk(small, &src).await.unwrap();
        assert!(small_chunks.len() > big_chunks.len());
    }
}
