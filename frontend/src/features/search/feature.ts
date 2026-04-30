import { execSync } from 'node:child_process';
import type { BlogFeature } from '../_types';

export function createSearchFeature(enabled: boolean): BlogFeature {
	return {
		name: 'search',
		enabled,
		postbuild: () => {
			execSync('pagefind --site dist', { stdio: 'inherit' });
		}
	};
}
