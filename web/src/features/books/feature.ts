import type { BlogFeature } from '../_types';

export function createBooksFeature(enabled: boolean): BlogFeature {
	return {
		name: 'books',
		enabled,
		routes: [
			{
				pattern: '/books',
				entrypoint: './src/features/books/route.astro'
			}
		],
		sitemapExclude(pathname) {
			return pathname === '/books' || pathname.startsWith('/books/');
		}
	};
}
