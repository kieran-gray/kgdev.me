import { getCollection, type CollectionEntry } from 'astro:content';
import { sortPostsByDate } from '../format/date';

export type PostEntry = CollectionEntry<'posts'> & { minutesRead?: string };

/**
 * Loads all posts and attaches `minutesRead` (injected by the remarkReadingTime
 * plugin) as a sidecar field. Sorted by pinned + pubDate desc.
 */
export async function getAllPosts(): Promise<PostEntry[]> {
	const entries: CollectionEntry<'posts'>[] = await getCollection('posts');
	const withMeta: PostEntry[] = await Promise.all(
		entries.map(async (entry) => {
			const { remarkPluginFrontmatter } = await entry.render();
			return Object.assign(entry, {
				minutesRead: remarkPluginFrontmatter?.minutesRead as string | undefined
			});
		})
	);
	withMeta.sort(sortPostsByDate);
	return withMeta;
}
