import { execSync } from 'node:child_process';
import { features } from '../src/config/features.mjs';

if (features.search.enabled) {
	execSync('pagefind --site dist', { stdio: 'inherit' });
}

