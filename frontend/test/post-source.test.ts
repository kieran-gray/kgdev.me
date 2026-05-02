import assert from 'node:assert/strict';
import { describe, test, vi, beforeEach } from 'vitest';
import { buildPostSourcePayload, stripFrontmatter } from '@/lib/content/post-source';
import { markdownToPlainText } from '@/lib/markdown/plainText';
import { getCollection } from 'astro:content';
import { PathOrFileDescriptor, readFileSync } from 'node:fs';

vi.mock('astro:content', () => ({
	getCollection: vi.fn()
}));

vi.mock('node:fs', async (importOriginal) => {
	const original = await importOriginal<typeof import('node:fs')>();
	return {
		...original,
		readFileSync: vi.fn()
	};
});

describe('post-source helpers', () => {
	beforeEach(() => {
		vi.mocked(getCollection).mockReset();
		vi.mocked(readFileSync).mockReset();
	});

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

	test('buildPostSourcePayload returns canonical metadata and normalized content', async () => {
		const entry = {
			slug: 'blog-view-counter',
			data: {
				title: 'A WebSocket-powered view counter with Cloudflare Durable Objects',
				description: 'A small WebSocket-driven view counter on Cloudflare Durable Objects in Rust.',
				excerpt:
					"I want a live view counter on each of my blog posts. Up when someone joins, down when they leave, no reload, and totals saved between sessions. Here's how I built it on Cloudflare Durable Objects in Rust.",
				author: 'Kieran Gray',
				pubDate: new Date('2026-04-27T00:00:00.000Z'),
				tags: ['cloudflare', 'rust'],
				glossaryTerms: []
			}
		};

		vi.mocked(getCollection).mockResolvedValue([]);
		vi.mocked(readFileSync).mockReturnValue('---\ntitle: Mock Post\n---\n\nBody content');

		const payload = await buildPostSourcePayload(entry as never);

		assert.equal(payload.slug, 'blog-view-counter');
		assert.equal(payload.canonicalUrl, 'https://kgdev.me/posts/blog-view-counter/');
		assert.equal(payload.rawMarkdownUrl, 'https://kgdev.me/posts/blog-view-counter.md');
		assert.equal(payload.jsonUrl, 'https://kgdev.me/api/posts/blog-view-counter.json');
		assert.equal(
			payload.contentHash,
			'0e5d4ffd56f0b01b6052fdf6a49a5b3bf1f145fb76afdc771abef37dd46c05d3'
		);
		assert.ok(payload.sourceMarkdown.startsWith('---\n'));
		assert.ok(!payload.markdownBody.startsWith('---\n'));
	});

	test('buildPostSourcePayload includes glossary terms', async () => {
		const entry = {
			slug: 'test-post',
			data: {
				title: 'Test Post',
				description: 'Test Description',
				excerpt: 'Test Excerpt',
				author: 'Test Author',
				pubDate: new Date('2026-04-27T00:00:00.000Z'),
				tags: [],
				glossaryTerms: ['astro']
			}
		};

		vi.mocked(getCollection).mockResolvedValue([
			{
				slug: 'astro',
				data: {
					term: 'Astro.js',
					sources: [{ title: 'Docs', url: 'https://docs.astro.build' }]
				}
			}
		] as never);

		vi.mocked(readFileSync).mockImplementation((path: PathOrFileDescriptor) => {
			if (typeof path === 'string' && path.includes('test-post.md')) {
				return '---\ntitle: Test Post\n---\n\nPost body';
			}
			if (typeof path === 'string' && path.includes('astro.md')) {
				return '---\nterm: Astro.js\nsources: [{ title: "Docs", url: "https://docs.astro.build" }]\n---\n\nAstro definition';
			}
			return '';
		});

		const payload = await buildPostSourcePayload(entry as never);

		assert.equal(payload.glossaryTerms.length, 1);
		const term = payload.glossaryTerms[0];
		assert.ok(term);
		assert.equal(term.slug, 'astro');
		assert.equal(term.term, 'Astro.js');
		assert.equal(term.definition, 'Astro definition');
		assert.equal(term.sources.length, 1);
	});
});
