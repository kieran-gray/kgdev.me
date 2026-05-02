pub(super) fn strip_frontmatter(source: &str) -> (Vec<char>, usize) {
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

pub(super) fn split_into_lines(body: &[char]) -> Vec<String> {
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

pub(super) fn fence_marker_of(line: &str) -> Option<String> {
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

pub(super) fn parse_heading(line: &str) -> Option<(usize, String)> {
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
}
