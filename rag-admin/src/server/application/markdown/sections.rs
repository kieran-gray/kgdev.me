use super::{heading_path, Block, Document, SectionBlock};

impl Document {
    pub fn sections(&self, cut_depth: usize) -> Vec<SectionBlock> {
        let mut sections = Vec::new();
        let mut heading_path: Vec<String> = Vec::new();
        let mut current: Vec<&Block> = Vec::new();

        for block in &self.blocks {
            if let Some(heading) = block.heading() {
                if heading.depth as usize <= cut_depth {
                    flush_section(&mut sections, &current, &heading_path);
                    current.clear();
                    heading_path::update_heading_path(&mut heading_path, heading);
                    current.push(block);
                    continue;
                }
                heading_path::update_heading_path(&mut heading_path, heading);
            }
            current.push(block);
        }

        flush_section(&mut sections, &current, &heading_path);
        sections
    }
}

fn flush_section(out: &mut Vec<SectionBlock>, current: &[&Block], heading_path: &[String]) {
    let Some(first) = current.first() else {
        return;
    };
    let last = current.last().unwrap();
    out.push(SectionBlock {
        text: heading_path::collect_block_text(current),
        char_start: first.span.char_start,
        char_end: last.span.char_end,
        heading: heading_path::join_heading_path(heading_path),
    });
}
