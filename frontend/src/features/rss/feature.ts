import type { BlogFeature } from '../_types';

export function createRssFeature(enabled: boolean): BlogFeature {
	return {
		name: 'rss',
		enabled,
		routes: [
			{
				pattern: '/rss.xml',
				entrypoint: './src/features/rss/rss.xml.ts',
				prerender: true
			}
		],
		sitemapExclude: (pathname) => pathname === '/rss.xml'
	};
}
