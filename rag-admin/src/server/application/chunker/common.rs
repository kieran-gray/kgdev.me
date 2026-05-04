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

