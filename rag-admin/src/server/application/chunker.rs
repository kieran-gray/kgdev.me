const TARGET_CHARS: usize = 1600;
const OVERLAP_CHARS: usize = 240;
const MIN_CHARS: usize = 320;

#[derive(Debug, Clone)]
pub struct ChunkOutput {
    pub chunk_id: u32,
    pub heading: String,
    pub text: String,
    pub char_start: u32,
    pub char_end: u32,
}

#[derive(Debug, Clone)]
struct Segment {
    text: String,
    char_start: usize,
    char_end: usize,
    heading: String,
    atomic: bool,
}

pub fn chunk(source: &str) -> Vec<ChunkOutput> {
    let (body_chars, body_offset) = strip_frontmatter(source);
    let raw = parse_segments(&body_chars);
    let packed = pack_segments(raw);
    let split = split_oversized(packed);

    split
        .into_iter()
        .filter(|s| !s.text.trim().is_empty())
        .enumerate()
        .map(|(i, s)| ChunkOutput {
            chunk_id: i as u32,
            heading: s.heading,
            text: s.text.trim().to_string(),
            char_start: (s.char_start + body_offset) as u32,
            char_end: (s.char_end + body_offset) as u32,
        })
        .collect()
}

pub fn strip_frontmatter(source: &str) -> (Vec<char>, usize) {
    let chars: Vec<char> = source.chars().collect();
    let prefix = ['-', '-', '-', '\n'];
    if chars.len() < 4 || chars[..4] != prefix {
        return (chars, 0);
    }
    let needle = ['\n', '-', '-', '-', '\n'];
    let mut end_idx: Option<usize> = None;
    let max_start = chars.len().saturating_sub(needle.len());
    let mut i = 4usize;
    while i <= max_start {
        if chars[i..i + needle.len()] == needle {
            end_idx = Some(i);
            break;
        }
        i += 1;
    }
    let Some(end) = end_idx else {
        return (chars, 0);
    };
    let body_offset = end + 5;
    let body = chars[body_offset..].to_vec();
    (body, body_offset)
}

fn split_into_lines(body: &[char]) -> Vec<String> {
    let mut lines = Vec::new();
    let mut buf = String::new();
    for &c in body {
        if c == '\n' {
            lines.push(std::mem::take(&mut buf));
        } else {
            buf.push(c);
        }
    }
    lines.push(buf);
    lines
}

fn fence_marker_of(line: &str) -> Option<String> {
    let mut chars = line.chars();
    let first = chars.next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let mut count = 1usize;
    for c in chars {
        if c == first {
            count += 1;
        } else {
            break;
        }
    }
    if count >= 3 {
        Some(std::iter::repeat_n(first, count).collect())
    } else {
        None
    }
}

fn parse_heading(line: &str) -> Option<(usize, String)> {
    let bytes = line.as_bytes();
    let mut depth = 0usize;
    while depth < bytes.len() && bytes[depth] == b'#' {
        depth += 1;
    }
    if depth == 0 || depth > 6 {
        return None;
    }
    let after = &line[depth..];
    let mut iter = after.chars();
    let first = iter.next()?;
    if !first.is_whitespace() {
        return None;
    }
    let text = after.trim().to_string();
    if text.is_empty() {
        return None;
    }
    Some((depth, text))
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

fn pack_segments(segments: Vec<Segment>) -> Vec<Segment> {
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
        if merged_text.chars().count() <= TARGET_CHARS {
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

fn split_oversized(segments: Vec<Segment>) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    for seg in segments {
        let text_chars: Vec<char> = seg.text.chars().collect();
        if seg.atomic || text_chars.len() <= TARGET_CHARS + OVERLAP_CHARS {
            out.push(seg);
            continue;
        }

        let mut start = 0usize;
        let total = text_chars.len();
        loop {
            if start >= total {
                break;
            }
            let end = (start + TARGET_CHARS).min(total);
            let mut break_at = end;
            if end < total {
                let window = &text_chars[start..end];
                let last_para = last_double_newline(window);
                let last_sent = last_sentence_break(window);
                if let Some(p) = last_para {
                    if p > TARGET_CHARS / 2 {
                        break_at = start + p;
                    } else if let Some(s) = last_sent {
                        if s > TARGET_CHARS / 2 {
                            break_at = start + s + 1;
                        }
                    }
                } else if let Some(s) = last_sent {
                    if s > TARGET_CHARS / 2 {
                        break_at = start + s + 1;
                    }
                }
            }
            let raw_piece: String = text_chars[start..break_at].iter().collect();
            let piece = raw_piece.trim().to_string();
            let piece_len = piece.chars().count();
            if !piece.is_empty() {
                let last_atomic = out.last().map(|s| s.atomic).unwrap_or(true);
                if piece_len < MIN_CHARS && !out.is_empty() && !last_atomic {
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
            let candidate = break_at.saturating_sub(OVERLAP_CHARS);
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

    #[test]
    fn strip_frontmatter_no_block() {
        let (chars, off) = strip_frontmatter("no frontmatter at all");
        assert_eq!(chars.iter().collect::<String>(), "no frontmatter at all");
        assert_eq!(off, 0);
    }

    #[test]
    fn strip_frontmatter_yaml_block() {
        let src = "---\ntitle: 'x'\n---\nbody here";
        let (chars, off) = strip_frontmatter(src);
        let body: String = chars.iter().collect();
        assert_eq!(body, "body here");
        let src_chars: Vec<char> = src.chars().collect();
        let reslice: String = src_chars[off..].iter().collect();
        assert_eq!(reslice, "body here");
    }

    #[test]
    fn chunk_yields_sequential_ids() {
        let src = "## A\nfirst paragraph.\n\n## B\nsecond paragraph.";
        let chunks = chunk(src);
        assert!(!chunks.is_empty());
        for (i, c) in chunks.iter().enumerate() {
            assert_eq!(c.chunk_id as usize, i);
        }
    }

    #[test]
    fn chunk_preserves_heading_path() {
        let src = "# Top\n\nintro\n\n## Sub\n\ndetail";
        let chunks = chunk(src);
        let headings: Vec<&str> = chunks.iter().map(|c| c.heading.as_str()).collect();
        assert!(headings.iter().any(|h| h.contains("Top")));
        assert!(headings.iter().any(|h| h.contains("Sub")));
    }

    #[test]
    fn chunk_does_not_split_inside_fence() {
        let code: String = (0..200)
            .map(|i| format!("let x{i} = {i};"))
            .collect::<Vec<_>>()
            .join("\n");
        let src = format!("## Code\n\n```rust\n{code}\n```\n");
        let chunks = chunk(&src);
        for c in chunks.iter().filter(|c| c.text.contains("```")) {
            let opens = c.text.matches("```").count();
            assert_eq!(opens % 2, 0, "chunk {} has unbalanced fences", c.chunk_id);
        }
    }
}
