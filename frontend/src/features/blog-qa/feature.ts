import type { BlogFeature } from '../_types';

export function createBlogQaFeature(enabled: boolean): BlogFeature {
	return {
		name: 'blogQa',
		enabled
	};
}
