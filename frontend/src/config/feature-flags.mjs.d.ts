export interface FeatureFlag {
	enabled: boolean;
}

export interface FeatureFlags {
	search: FeatureFlag;
	viewCounter: FeatureFlag;
	contact: FeatureFlag;
	og: FeatureFlag;
	mermaid: FeatureFlag;
	rss: FeatureFlag;
	projects: FeatureFlag;
	books: FeatureFlag;
}

export function readFlag(
	env: Record<string, string | undefined>,
	name: string,
	defaultValue: boolean
): boolean;

export function getFeatureFlags(env: Record<string, string | undefined>): FeatureFlags;

export const features: FeatureFlags;
