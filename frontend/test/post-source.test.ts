import assert from 'node:assert/strict';
import { describe, test, vi } from 'vitest';
import { createHash } from 'node:crypto';
import { buildPostSourcePayload, stripFrontmatter } from '@/lib/content/post-source';
import { markdownToPlainText } from '@/lib/markdown/plainText';

vi.mock('astro:content', () => ({
	getCollection: vi.fn()
}));

describe('post-source helpers', () => {
	test('stripFrontmatter removes leading yaml block', () => {
		const source = '---\ntitle: test\n---\n\n# Heading\nBody';
		assert.equal(stripFrontmatter(source), '\n# Heading\nBody');
	});

	test('markdownToPlainText uses markdown semantics', () => {
		const markdown = [
			'# Heading',
			'',
			'Paragraph with [a link](https://example.com), `inline code`, and **bold text**.',
			'',
			'- first item',
			'- second item',
			'',
			'> quoted line',
			'',
			'```ts',
			'const answer = 42;',
			'```'
		].join('\n');

		const text = markdownToPlainText(markdown);
		assert.match(text, /Heading/);
		assert.match(text, /Paragraph with a link, inline code, and bold text\./);
		assert.match(text, /first item/);
		assert.match(text, /quoted line/);
		assert.match(text, /const answer = 42;/);
		assert.doesNotMatch(text, /\[a link\]/);
		assert.doesNotMatch(text, /\*\*bold text\*\*/);
	});

	test('buildPostSourcePayload returns canonical metadata and normalized content', () => {
		const entry = {
			slug: 'blog-view-counter',
			data: {
				title: 'A WebSocket-powered view counter with Cloudflare Durable Objects',
				description: 'A small WebSocket-driven view counter on Cloudflare Durable Objects in Rust.',
				excerpt:
					"I want a live view counter on each of my blog posts. Up when someone joins, down when they leave, no reload, and totals saved between sessions. Here's how I built it on Cloudflare Durable Objects in Rust.",
				author: 'Kieran Gray',
				pubDate: new Date('2026-04-27T00:00:00.000Z'),
				tags: ['cloudflare', 'rust']
			}
		};

		const payload = buildPostSourcePayload(entry as never);
		const expectedHash = createHash('sha256').update(payload.sourceMarkdown).digest('hex');

		assert.equal(payload.slug, 'blog-view-counter');
		assert.equal(payload.canonicalUrl, 'https://kgdev.me/posts/blog-view-counter/');
		assert.equal(payload.rawMarkdownUrl, 'https://kgdev.me/posts/blog-view-counter.md');
		assert.equal(payload.jsonUrl, 'https://kgdev.me/api/posts/blog-view-counter.json');
		assert.equal(payload.contentHash, expectedHash);
		assert.ok(payload.sourceMarkdown.startsWith('---\n'));
		assert.ok(!payload.markdownBody.startsWith('---\n'));
		assert.match(payload.plainText, /I want a live view counter on each of my blog posts\./);
		assert.match(payload.plainText, /The architecture/);
	});
});
