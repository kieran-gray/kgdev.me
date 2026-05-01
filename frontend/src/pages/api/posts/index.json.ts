import type { APIRoute } from 'astro';
import { getAllPostSourcePayloads } from '@/lib/content/post-source';

export const GET: APIRoute = async () => {
	const posts = await getAllPostSourcePayloads();

	return Response.json({
		posts: posts.map((post) => ({
			slug: post.slug,
			title: post.title,
			description: post.description,
			excerpt: post.excerpt,
			author: post.author,
			publishedAt: post.publishedAt,
			tags: post.tags,
			canonicalUrl: post.canonicalUrl,
			rawMarkdownUrl: post.rawMarkdownUrl,
			jsonUrl: post.jsonUrl,
			contentHash: post.contentHash
		}))
	});
};
