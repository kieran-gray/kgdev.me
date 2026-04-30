export function getSelectedWranglerEnv(env?: Record<string, string | undefined>): string | null;

export function readWranglerConfig(configPath?: string): Record<string, unknown>;

export function getWranglerPublicVars(options?: {
	env?: Record<string, string | undefined>;
	configPath?: string;
}): Record<string, string>;
