import type { BlogFeature } from '../_types';

export function createContactFeature(enabled: boolean): BlogFeature {
	return {
		name: 'contact',
		enabled,
		routes: [
			{
				pattern: '/contact',
				entrypoint: './src/features/contact/route.astro',
				prerender: true
			}
		]
	};
}
