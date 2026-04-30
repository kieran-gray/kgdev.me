/// <reference path="../.astro/types.d.ts" />
/// <reference types="astro/client" />

interface ImportMetaEnv {
	readonly PUBLIC_FEATURE_SEARCH: string;
	readonly PUBLIC_FEATURE_VIEW_COUNTER: string;
	readonly PUBLIC_FEATURE_CONTACT: string;
	readonly PUBLIC_FEATURE_OG: string;
	readonly PUBLIC_FEATURE_MERMAID: string;
	readonly PUBLIC_FEATURE_RSS: string;
	readonly PUBLIC_FEATURE_BOOKS: string;
	readonly PUBLIC_FEATURE_PROJECTS: string;
	readonly PUBLIC_VIEW_COUNTER_URL: string;
	readonly PUBLIC_CONTACT_URL: string;
	readonly PUBLIC_TURNSTILE_SITE_KEY: string;
}

interface ImportMeta {
	readonly env: ImportMetaEnv;
}
