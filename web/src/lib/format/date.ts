import type { CollectionEntry } from 'astro:content';

export const formatDate = (pubDate: string | Date) => {
	const options: Intl.DateTimeFormatOptions = {
		weekday: 'short',
		year: 'numeric',
		month: 'long',
		day: 'numeric'
	};

	return new Date(pubDate).toLocaleDateString('en-US', options);
};

export const sortPostsByDate = (a: CollectionEntry<'posts'>, b: CollectionEntry<'posts'>) => {
	const isPinnedA = a.data.isPinned === true;
	const isPinnedB = b.data.isPinned === true;

	if (isPinnedA && !isPinnedB) return -1;
	if (!isPinnedA && isPinnedB) return 1;

	const pubDateA = a.data.pubDate.getTime();
	const pubDateB = b.data.pubDate.getTime();
	if (pubDateA < pubDateB) return 1;
	if (pubDateA > pubDateB) return -1;
	return 0;
};
