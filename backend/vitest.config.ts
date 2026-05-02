import { defineWorkersConfig } from '@cloudflare/vitest-pool-workers/config';

export default defineWorkersConfig(async () => {
	return {
		test: {
			poolOptions: {
				workers: {
					remoteBindings: false,
					wrangler: { configPath: './wrangler.jsonc' },
					miniflare: {
						bindings: {
							ENVIRONMENT: 'test',
							CLOUDFLARE_ACCOUNT_ID: 'test-account-id',
							CLOUDFLARE_EMAIL_API_TOKEN: 'test-api-token',
							CLOUDFLARE_VECTORIZE_API_TOKEN: 'test-vectorize-api-token',
							CLOUDFLARE_SITEVERIFY_URL: 'https://test.kgdev.me/turnstile/v0/siteverify',
							CLOUDFLARE_TURNSTILE_SECRET_KEY: 'test-secret-key',
							ALLOWED_ORIGINS: 'http://localhost:5173,http://localhost:5174,http://localhost:5175',
							ALLOWED_BLOG_PATHS: 'my-post',
							DESTINATION_EMAIL: 'test@email.com',
							EMBEDDING_MODEL: "test-embedding-model"
						}
					}
				}
			}
		}
	};
});
