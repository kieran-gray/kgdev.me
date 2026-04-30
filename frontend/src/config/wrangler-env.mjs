import fs from 'node:fs';
import path from 'node:path';

const DEFAULT_WRANGLER_CONFIG = path.resolve(import.meta.dirname, '../../wrangler.jsonc');

export function getSelectedWranglerEnv(env = process.env) {
	return env.CLOUDFLARE_ENV?.trim() || null;
}

export function readWranglerConfig(configPath = DEFAULT_WRANGLER_CONFIG) {
	const source = fs.readFileSync(configPath, 'utf8');

	try {
		return JSON.parse(source);
	} catch (error) {
		throw new Error(
			`Failed to parse ${path.basename(configPath)} as JSON. Keep the file JSON-compatible so Astro can read build-time vars.`,
			{ cause: error }
		);
	}
}

export function getWranglerPublicVars({
	env = process.env,
	configPath = DEFAULT_WRANGLER_CONFIG
} = {}) {
	const wranglerConfig = readWranglerConfig(configPath);
	const envName = getSelectedWranglerEnv(env);
	const selectedConfig = envName ? wranglerConfig.env?.[envName] : wranglerConfig;

	if (envName && !selectedConfig) {
		throw new Error(`CLOUDFLARE_ENV="${envName}" is not defined in wrangler.jsonc.`);
	}

	const wranglerVars = selectedConfig?.vars ?? {};
	const processOverrides = Object.fromEntries(
		Object.entries(env).filter(([key, value]) => key.startsWith('PUBLIC_') && value != null)
	);

	return {
		...wranglerVars,
		...processOverrides
	};
}
