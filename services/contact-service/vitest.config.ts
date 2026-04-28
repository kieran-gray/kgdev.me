import { defineWorkersConfig, readD1Migrations } from '@cloudflare/vitest-pool-workers/config';

export default defineWorkersConfig(async () => {
	return {
		test: {
			poolOptions: {
				workers: {
					wrangler: { configPath: './wrangler.jsonc' },
					miniflare: {
						bindings: {
							ENVIRONMENT: 'test',
							CLOUDFLARE_ACCOUNT_ID: 'test-account-id',
							CLOUDFLARE_EMAIL_API_TOKEN: 'test-api-token',
							CLOUDFLARE_SITEVERIFY_URL: 'https://test.kgdev.me/turnstile/v0/siteverify',
							CLOUDFLARE_TURNSTILE_SECRET_KEY: 'test-secret-key',
							ALLOWED_ORIGINS: 'http://localhost:5173,http://localhost:5174,http://localhost:5175',
							DESTINATION_EMAIL: 'test@email.com'
						}
					}
				}
			}
		}
	};
});
