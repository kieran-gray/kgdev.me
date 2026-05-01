import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { chunk, stripFrontmatter } from './chunker.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const postsDir = resolve(here, '../../src/content/posts');

test('stripFrontmatter removes YAML block and reports offset', () => {
	const src = "---\ntitle: 'x'\n---\nbody here";
	const { body, bodyOffset } = stripFrontmatter(src);
	assert.equal(body, 'body here');
	assert.equal(src.slice(bodyOffset), 'body here');
});

test('stripFrontmatter is a no-op when no frontmatter', () => {
	const src = 'no frontmatter at all';
	const { body, bodyOffset } = stripFrontmatter(src);
	assert.equal(body, src);
	assert.equal(bodyOffset, 0);
});

test('chunk yields sequential chunk_ids starting at 0', () => {
	const src = '## A\nfirst paragraph.\n\n## B\nsecond paragraph.';
	const chunks = chunk(src);
	assert.ok(chunks.length >= 1);
	chunks.forEach((c, i) => assert.equal(c.chunk_id, i));
});

test('chunk preserves heading path', () => {
	const src = ['# Top', '', 'intro', '', '## Sub', '', 'detail'].join('\n');
	const chunks = chunk(src);
	const headings = chunks.map((c) => c.heading);
	assert.ok(headings.some((h) => h.includes('Top')));
	assert.ok(headings.some((h) => h.includes('Sub')));
});

test('chunk does not split inside a fenced code block', () => {
	const code = Array.from({ length: 200 }, (_, i) => `let x${i} = ${i};`).join('\n');
	const src = `## Code\n\n\`\`\`rust\n${code}\n\`\`\`\n`;
	const chunks = chunk(src);
	const codeChunks = chunks.filter((c) => c.text.includes('```'));
	for (const c of codeChunks) {
		const opens = (c.text.match(/```/g) || []).length;
		assert.equal(opens % 2, 0, `chunk ${c.chunk_id} has unbalanced fences`);
	}
});

test('chunk char_start/char_end map back to source', () => {
	const src = readFileSync(resolve(postsDir, 'blog-view-counter.md'), 'utf8');
	const chunks = chunk(src);
	for (const c of chunks) {
		const slice = src.slice(c.char_start, c.char_end);
		assert.ok(slice.includes(c.text.split('\n')[0].slice(0, 20)));
	}
});

test('real posts produce non-empty chunks within size budget', () => {
	const slugs = [
		'blog-view-counter',
		'event-sourcing-cloudflare',
		'quest-exactly-once-p1',
		'quest-exactly-once-p2',
	];
	for (const slug of slugs) {
		const src = readFileSync(resolve(postsDir, `${slug}.md`), 'utf8');
		const chunks = chunk(src);
		assert.ok(chunks.length > 0, `${slug} produced no chunks`);
		for (const c of chunks) {
			assert.ok(c.text.length > 0, `${slug} chunk ${c.chunk_id} empty`);
			assert.ok(
				c.text.length < 4000,
				`${slug} chunk ${c.chunk_id} too large: ${c.text.length}`,
			);
		}
	}
});
