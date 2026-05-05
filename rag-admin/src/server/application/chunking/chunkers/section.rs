use async_trait::async_trait;

use super::common::{fence_marker_of, parse_heading, split_into_lines};
use crate::{
    server::application::{
        chunking::{ChunkOutput, TextChunker},
        AppError,
    },
    shared::{ChunkStrategy, ChunkingConfig},
};

const SECTION_CUT_DEPTH: usize = 3;

#[derive(Debug, Clone)]
struct Section {
    text: String,
    char_start: usize,
    char_end: usize,
    heading: String,
}

pub struct SectionChunker;

#[async_trait]
impl TextChunker for SectionChunker {
    fn strategy(&self) -> ChunkStrategy {
        ChunkStrategy::Section
    }

    async fn chunk(
        &self,
        config: ChunkingConfig,
        source: &str,
    ) -> Result<Vec<ChunkOutput>, AppError> {
        let max_chars = config.max_section_chars();
        let body_chars: Vec<char> = source.chars().collect();
        let sections = parse_sections(&body_chars);
        let split = sections
            .into_iter()
            .flat_map(|s| split_oversized(s, max_chars));

        Ok(split
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

fn parse_sections(body: &[char]) -> Vec<Section> {
    let lines = split_into_lines(body);
    let mut sections: Vec<Section> = Vec::new();
    let mut cursor: usize = 0;
    let mut heading_path: Vec<String> = Vec::new();

    let mut buf: Vec<String> = Vec::new();
    let mut buf_start: usize = 0;

    let mut in_fence = false;
    let mut fence_marker = String::new();

    let flush = |buf: &mut Vec<String>,
                 buf_start: &mut usize,
                 heading_path: &[String],
                 end_offset: usize,
                 sections: &mut Vec<Section>| {
        if buf.is_empty() {
            return;
        }
        sections.push(Section {
            text: buf.join("\n"),
            char_start: *buf_start,
            char_end: end_offset,
            heading: heading_path
                .iter()
                .filter(|s| !s.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(" > "),
        });
        buf.clear();
    };

    for line in lines.iter() {
        let line_start = cursor;
        let line_len = line.chars().count();
        let line_with_newline = line_start + line_len + 1;

        let fence_match = fence_marker_of(line);

        if in_fence {
            if buf.is_empty() {
                buf_start = line_start;
            }
            buf.push(line.clone());
            if fence_match.is_some() && line.starts_with(&fence_marker) {
                in_fence = false;
                fence_marker.clear();
            }
            cursor = line_with_newline;
            continue;
        }

        if let Some(marker) = fence_match {
            if buf.is_empty() {
                buf_start = line_start;
            }
            buf.push(line.clone());
            in_fence = true;
            fence_marker = marker;
            cursor = line_with_newline;
            continue;
        }

        if let Some((depth, text)) = parse_heading(line) {
            if depth <= SECTION_CUT_DEPTH {
                flush(
                    &mut buf,
                    &mut buf_start,
                    &heading_path,
                    line_start,
                    &mut sections,
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
            heading_path.truncate(depth - 1);
            while heading_path.len() < depth {
                heading_path.push(String::new());
            }
            heading_path[depth - 1] = text;
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
        &heading_path,
        cursor,
        &mut sections,
    );

    sections
}

fn split_oversized(section: Section, max_chars: usize) -> Vec<Section> {
    let chars: Vec<char> = section.text.chars().collect();
    if chars.len() <= max_chars {
        return vec![section];
    }

    let mut out: Vec<Section> = Vec::new();
    let total = chars.len();
    let mut start = 0usize;
    while start < total {
        let end = (start + max_chars).min(total);
        let break_at = if end < total {
            last_double_newline(&chars[start..end])
                .map(|p| start + p)
                .unwrap_or(end)
        } else {
            end
        };
        let break_at = if break_at <= start { end } else { break_at };
        let piece: String = chars[start..break_at].iter().collect();
        out.push(Section {
            text: piece,
            char_start: section.char_start + start,
            char_end: section.char_start + break_at,
            heading: section.heading.clone(),
        });
        start = break_at;
        while start < total && chars[start] == '\n' {
            start += 1;
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> ChunkingConfig {
        ChunkingConfig::default()
    }

    #[tokio::test]
    async fn one_chunk_per_h2() {
        let src = "## A\n\nfirst paragraph.\n\n## B\n\nsecond paragraph.";
        let chunker = SectionChunker {};
        let chunks = chunker.chunk(cfg(), src).await.unwrap();
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].text.contains("first paragraph"));
        assert!(chunks[1].text.contains("second paragraph"));
    }

    #[tokio::test]
    async fn h4_does_not_cut() {
        let src = "## Top\n\nintro\n\n#### Deep\n\nstill same chunk.";
        let chunker = SectionChunker {};
        let chunks = chunker.chunk(cfg(), src).await.unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("Deep"));
        assert!(chunks[0].text.contains("still same chunk"));
    }

    #[tokio::test]
    async fn fenced_heading_does_not_cut() {
        let src = "## A\n\n```md\n## not a real heading\n```\n\nbody";
        let chunker = SectionChunker {};
        let chunks = chunker.chunk(cfg(), src).await.unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("## not a real heading"));
    }

    #[tokio::test]
    async fn oversized_section_splits() {
        let max = cfg().max_section_chars as usize;
        let para = "x".repeat(max / 2);
        let src = format!("## Big\n\n{para}\n\n{para}\n\n{para}");
        let chunker = SectionChunker {};
        let chunks = chunker.chunk(cfg(), &src).await.unwrap();
        assert!(
            chunks.len() > 1,
            "expected fallback split, got {} chunks",
            chunks.len()
        );
        for c in &chunks {
            assert_eq!(c.heading, "Big");
        }
    }

    #[tokio::test]
    async fn smaller_max_section_chars_increases_splits() {
        let chunker = SectionChunker {};

        let para = "x".repeat(2000);
        let src = format!("## Big\n\n{para}\n\n{para}\n\n{para}");
        let big = ChunkingConfig {
            max_section_chars: 8000,
            ..Default::default()
        };
        let small = ChunkingConfig {
            max_section_chars: 2500,
            ..Default::default()
        };
        let big_chunks = chunker.chunk(big, &src).await.unwrap();
        let small_chunks = chunker.chunk(small, &src).await.unwrap();
        assert!(small_chunks.len() > big_chunks.len());
    }

    #[tokio::test]
    async fn heading_path_preserved() {
        let src = "# Top\n\nintro\n\n## Sub\n\ndetail";
        let chunker = SectionChunker {};
        let chunks = chunker.chunk(cfg(), src).await.unwrap();
        let headings: Vec<&str> = chunks.iter().map(|c| c.heading.as_str()).collect();
        assert!(headings.iter().any(|h| h.contains("Top")));
        assert!(headings.iter().any(|h| h.contains("Sub")));
        assert!(headings.iter().any(|h| h.contains("Top > Sub")));
    }

    #[tokio::test]
    async fn skipped_heading_levels() {
        let src = "# Top\n\n### Deep";
        let chunker = SectionChunker {};
        let chunks = chunker.chunk(cfg(), src).await.unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].heading, "Top");
        assert_eq!(chunks[1].heading, "Top > Deep");
    }
}
