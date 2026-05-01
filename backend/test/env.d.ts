import type { Ai } from '@cloudflare/workers-types';

declare module 'cloudflare:test' {
	interface ProvidedEnv {
		AI: Ai;
	}
}
