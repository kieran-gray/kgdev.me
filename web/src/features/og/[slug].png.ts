import type { APIRoute, GetStaticPaths } from 'astro';
import { getCollection, type CollectionEntry } from 'astro:content';
import { renderPostCard } from './card';

export const getStaticPaths: GetStaticPaths = async () => {
	const posts: CollectionEntry<'posts'>[] = await getCollection('posts');
	return posts.map((entry) => ({
		params: { slug: entry.slug },
		props: {
			title: entry.data.title,
			tags: entry.data.tags ?? []
		}
	}));
};

export const GET: APIRoute = async ({ props }) => {
	const { title, tags } = props as { title: string; tags: string[] };
	const png = await renderPostCard(title, tags);
	return new Response(png, {
		headers: {
			'Content-Type': 'image/png',
			'Cache-Control': 'public, max-age=31536000, immutable'
		}
	});
};
