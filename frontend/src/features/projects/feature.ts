import type { BlogFeature } from '../_types';

export function createProjectsFeature(enabled: boolean): BlogFeature {
	return {
		name: 'projects',
		enabled,
		routes: [
			{
				pattern: '/projects',
				entrypoint: './src/features/projects/routes/index.astro'
			},
			{
				pattern: '/projects/[slug]',
				entrypoint: './src/features/projects/routes/[slug].astro'
			}
		],
		sitemapExclude(pathname) {
			return pathname === '/projects' || pathname.startsWith('/projects/');
		}
	};
}
