import type { BlogFeature } from '../_types';

export function createViewCounterFeature(enabled: boolean): BlogFeature {
	return {
		name: 'viewCounter',
		enabled
	};
}
