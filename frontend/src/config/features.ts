// @ts-ignore Typed through the exported interfaces below; source stays .mjs for Node scripts.
import { getFeatureFlags } from './feature-flags.mjs';

export interface FeatureFlag {
	enabled: boolean;
}

export interface Features {
	search: FeatureFlag;
	viewCounter: FeatureFlag;
	contact: FeatureFlag;
	og: FeatureFlag;
	mermaid: FeatureFlag;
	rss: FeatureFlag;
	projects: FeatureFlag;
	books: FeatureFlag;
}

const typedFlags = getFeatureFlags(import.meta.env) as Features;

export interface FeatureRuntime {
	flags: Features;
	viewCounter: { wsUrl: string };
	contact: { endpoint: string; turnstileSiteKey: string };
}

export const featureRuntime: FeatureRuntime = {
	flags: typedFlags,
	viewCounter: {
		wsUrl: import.meta.env.PUBLIC_VIEW_COUNTER_URL ?? ''
	},
	contact: {
		endpoint: import.meta.env.PUBLIC_CONTACT_URL ?? '',
		turnstileSiteKey: import.meta.env.PUBLIC_TURNSTILE_SITE_KEY ?? ''
	}
};

export const features: Features = typedFlags;
