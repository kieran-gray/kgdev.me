/// <reference path="../.astro/types.d.ts" />
/// <reference types="astro/client" />

interface ImportMetaEnv {
	readonly PUBLIC_VIEW_COUNTER_URL: string;
}

interface ImportMeta {
	readonly env: ImportMetaEnv;
}
