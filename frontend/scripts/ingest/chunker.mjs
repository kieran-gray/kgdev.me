const TARGET_TOKENS = 400;
const OVERLAP_TOKENS = 60;
const MIN_TOKENS = 80;

const CHARS_PER_TOKEN = 4;

const targetChars = TARGET_TOKENS * CHARS_PER_TOKEN;
const overlapChars = OVERLAP_TOKENS * CHARS_PER_TOKEN;
const minChars = MIN_TOKENS * CHARS_PER_TOKEN;

export function stripFrontmatter(source) {
	if (!source.startsWith('---\n')) {
		return { body: source, bodyOffset: 0 };
	}
	const end = source.indexOf('\n---\n', 4);
	if (end === -1) {
		return { body: source, bodyOffset: 0 };
	}
	const bodyOffset = end + 5;
	return { body: source.slice(bodyOffset), bodyOffset };
}

function parseSegments(body) {
	const lines = body.split('\n');
	const segments = [];
	let cursor = 0;
	let headingPath = [];

	let buf = [];
	let bufStart = 0;
	let bufAtomic = false;

	const flush = (endOffset) => {
		if (buf.length === 0) return;
		segments.push({
			text: buf.join('\n'),
			charStart: bufStart,
			charEnd: endOffset,
			heading: headingPath.join(' > '),
			atomic: bufAtomic,
		});
		buf = [];
		bufAtomic = false;
	};

	let inFence = false;
	let fenceMarker = '';

	for (const line of lines) {
		const lineStart = cursor;
		const lineEnd = cursor + line.length;
		const lineWithNewline = lineEnd + 1;
		const fenceMatch = line.match(/^(```+|~~~+)/);

		if (!inFence && fenceMatch) {
			flush(lineStart);
			inFence = true;
			fenceMarker = fenceMatch[1];
			bufStart = lineStart;
			bufAtomic = true;
			buf.push(line);
			cursor = lineWithNewline;
			continue;
		}

		if (inFence) {
			buf.push(line);
			if (fenceMatch && line.startsWith(fenceMarker)) {
				inFence = false;
				fenceMarker = '';
				flush(lineWithNewline);
			}
			cursor = lineWithNewline;
			continue;
		}

		const headingMatch = line.match(/^(#{1,6})\s+(.+?)\s*$/);
		if (headingMatch) {
			flush(lineStart);
			const depth = headingMatch[1].length;
			const text = headingMatch[2];
			headingPath = headingPath.slice(0, depth - 1);
			headingPath[depth - 1] = text;
			bufStart = lineStart;
			buf.push(line);
			cursor = lineWithNewline;
			continue;
		}

		if (buf.length === 0) bufStart = lineStart;
		buf.push(line);
		cursor = lineWithNewline;
	}

	flush(cursor);
	return segments;
}

function packSegments(segments) {
	const packed = [];
	let current = null;

	for (const seg of segments) {
		if (!current) {
			current = { ...seg };
			continue;
		}

		if (current.atomic || seg.atomic || current.heading !== seg.heading) {
			packed.push(current);
			current = { ...seg };
			continue;
		}

		const merged = current.text + '\n' + seg.text;
		if (merged.length <= targetChars) {
			current.text = merged;
			current.charEnd = seg.charEnd;
			continue;
		}

		packed.push(current);
		current = { ...seg };
	}

	if (current) packed.push(current);
	return packed;
}

function splitOversized(segments) {
	const out = [];
	for (const seg of segments) {
		if (seg.atomic || seg.text.length <= targetChars + overlapChars) {
			out.push(seg);
			continue;
		}

		const text = seg.text;
		let start = 0;
		while (start < text.length) {
			const end = Math.min(start + targetChars, text.length);
			let breakAt = end;
			if (end < text.length) {
				const window = text.slice(start, end);
				const lastPara = window.lastIndexOf('\n\n');
				const lastSentence = window.search(/[.!?]\s+(?=[^.!?]*$)/);
				if (lastPara > targetChars / 2) breakAt = start + lastPara;
				else if (lastSentence > targetChars / 2) breakAt = start + lastSentence + 1;
			}
			const piece = text.slice(start, breakAt).trim();
			if (piece.length > 0) {
				if (piece.length < minChars && out.length > 0 && !out[out.length - 1].atomic) {
					const prev = out[out.length - 1];
					prev.text = prev.text + '\n\n' + piece;
					prev.charEnd = seg.charStart + breakAt;
				} else {
					out.push({
						text: piece,
						charStart: seg.charStart + start,
						charEnd: seg.charStart + breakAt,
						heading: seg.heading,
						atomic: false,
					});
				}
			}
			if (breakAt >= text.length) break;
			start = breakAt - overlapChars > start ? breakAt - overlapChars : breakAt;
		}
	}
	return out;
}

export function chunk(source) {
	const { body, bodyOffset } = stripFrontmatter(source);
	const raw = parseSegments(body);
	const packed = packSegments(raw);
	const split = splitOversized(packed);

	return split
		.filter((seg) => seg.text.trim().length > 0)
		.map((seg, i) => ({
			chunk_id: i,
			heading: seg.heading,
			text: seg.text.trim(),
			char_start: seg.charStart + bodyOffset,
			char_end: seg.charEnd + bodyOffset,
		}));
}
