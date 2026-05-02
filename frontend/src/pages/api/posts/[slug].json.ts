import type { APIRoute } from 'astro';
import { CollectionEntry, getCollection } from 'astro:content';
import { buildPostSourcePayload } from '@/lib/content/post-source';

export async function getStaticPaths() {
	const posts: CollectionEntry<'posts'>[] = await getCollection('posts');
	return posts.map((entry) => ({ params: { slug: entry.slug }, props: { entry } }));
}

export const GET: APIRoute = async ({ props }) => {
	const { entry } = props;
	const payload = await buildPostSourcePayload(entry);

	return Response.json(payload, {
		headers: {
			'cache-control': 'public, max-age=0, must-revalidate',
			etag: `"${payload.contentHash}"`
		}
	});
};
