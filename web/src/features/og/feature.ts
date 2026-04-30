import type { BlogFeature } from '../_types';

export function createOgFeature(enabled: boolean): BlogFeature {
	return {
		name: 'og',
		enabled,
		routes: [
			{
				pattern: '/og/default.png',
				entrypoint: './src/features/og/default.png.ts',
				prerender: true
			},
			{
				pattern: '/og/[slug].png',
				entrypoint: './src/features/og/[slug].png.ts',
				prerender: true
			}
		],
		sitemapExclude: (pathname) => pathname.startsWith('/og/')
	};
}
