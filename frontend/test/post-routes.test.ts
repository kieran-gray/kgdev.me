import assert from 'node:assert/strict';
import { beforeEach, describe, test, vi } from 'vitest';

const buildPostSourcePayload = vi.fn();
const getAllPostSourcePayloads = vi.fn();
const getCollection = vi.fn();

vi.mock('@/lib/content/post-source', () => ({
	buildPostSourcePayload,
	getAllPostSourcePayloads
}));

vi.mock('astro:content', () => ({
	getCollection
}));

describe('post endpoints', () => {
	beforeEach(() => {
		buildPostSourcePayload.mockReset();
		getAllPostSourcePayloads.mockReset();
		getCollection.mockReset();
	});

	test('markdown route emits raw markdown with cache headers', async () => {
		buildPostSourcePayload.mockResolvedValue({
			sourceMarkdown: '# Hello\n',
			contentHash: 'abc123'
		});

		const mod = await import('../src/pages/posts/[slug].md');
		const response = await mod.GET({ props: { entry: { slug: 'hello' } } } as never);

		assert.equal(await response.text(), '# Hello\n');
		assert.equal(response.headers.get('content-type'), 'text/markdown; charset=utf-8');
		assert.equal(response.headers.get('etag'), '"abc123"');
		assert.equal(response.headers.get('cache-control'), 'public, max-age=0, must-revalidate');
	});

	test('json route emits full payload with etag', async () => {
		buildPostSourcePayload.mockResolvedValue({
			slug: 'hello',
			contentHash: 'def456',
			sourceMarkdown: '# Hello\n',
			markdownBody: '# Hello',
			plainText: 'Hello'
		});

		const mod = await import('../src/pages/api/posts/[slug].json');
		const response = await mod.GET({ props: { entry: { slug: 'hello' } } } as never);

		assert.equal(response.headers.get('etag'), '"def456"');
		assert.deepEqual(await response.json(), {
			slug: 'hello',
			contentHash: 'def456',
			sourceMarkdown: '# Hello\n',
			markdownBody: '# Hello',
			plainText: 'Hello'
		});
	});

	test('index route emits summary rows for all posts', async () => {
		getAllPostSourcePayloads.mockResolvedValue([
			{
				slug: 'a',
				title: 'A',
				description: 'desc',
				excerpt: 'excerpt',
				author: 'Author',
				publishedAt: '2026-05-01T00:00:00.000Z',
				tags: ['rust'],
				canonicalUrl: 'https://kgdev.me/posts/a/',
				rawMarkdownUrl: 'https://kgdev.me/posts/a.md',
				jsonUrl: 'https://kgdev.me/api/posts/a.json',
				contentHash: 'hash-a'
			}
		]);

		const mod = await import('../src/pages/api/posts/index.json');
		const response = await mod.GET({} as never);

		assert.deepEqual(await response.json(), {
			posts: [
				{
					slug: 'a',
					title: 'A',
					description: 'desc',
					excerpt: 'excerpt',
					author: 'Author',
					publishedAt: '2026-05-01T00:00:00.000Z',
					tags: ['rust'],
					canonicalUrl: 'https://kgdev.me/posts/a/',
					rawMarkdownUrl: 'https://kgdev.me/posts/a.md',
					jsonUrl: 'https://kgdev.me/api/posts/a.json',
					contentHash: 'hash-a'
				}
			]
		});
	});

	test('route static paths are derived from the posts collection', async () => {
		getCollection.mockResolvedValue([{ slug: 'a' }, { slug: 'b' }]);

		const rawRoute = await import('../src/pages/posts/[slug].md');
		const jsonRoute = await import('../src/pages/api/posts/[slug].json');

		assert.deepEqual(await rawRoute.getStaticPaths(), [
			{ params: { slug: 'a' }, props: { entry: { slug: 'a' } } },
			{ params: { slug: 'b' }, props: { entry: { slug: 'b' } } }
		]);
		assert.deepEqual(await jsonRoute.getStaticPaths(), [
			{ params: { slug: 'a' }, props: { entry: { slug: 'a' } } },
			{ params: { slug: 'b' }, props: { entry: { slug: 'b' } } }
		]);
	});
});
