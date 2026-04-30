import { execSync } from 'node:child_process';
import { getFeatureFlags } from '../src/config/feature-flags.mjs';
import { getWranglerPublicVars } from '../src/config/wrangler-env.mjs';

if (getFeatureFlags(getWranglerPublicVars({ env: process.env })).search.enabled) {
	execSync('pagefind --site dist', { stdio: 'inherit' });
}
