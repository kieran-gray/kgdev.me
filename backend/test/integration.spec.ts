import { SELF, fetchMock, env } from 'cloudflare:test';
import { describe, it, expect, beforeAll, afterEach, vi } from 'vitest';

describe('Contact Us Worker', () => {
	beforeAll(async () => {
		fetchMock.activate();
		fetchMock.disableNetConnect();
	});

	afterEach(() => {
		vi.restoreAllMocks();
		fetchMock.assertNoPendingInterceptors();
	});

	it('responds with 404 for unknown routes', async () => {
		const response = await SELF.fetch('http://example.com/404');
		expect(response.status).toBe(404);
		expect(await response.text()).toBe('Not Found');
	});

	it('handles OPTIONS request for CORS preflight', async () => {
		const response = await SELF.fetch('http://example.com/api/v1/contact/', {
			method: 'OPTIONS',
			headers: {
				Origin: 'http://localhost:5173'
			}
		});

		expect(response.status).toBe(200);
		expect(response.headers.get('Access-Control-Allow-Origin')).toBe('http://localhost:5173');
		expect(response.headers.get('Access-Control-Allow-Methods')).toBe('GET, POST, OPTIONS');
		expect(response.headers.get('Access-Control-Allow-Headers')).toBe(
			'Content-Type, Authorization'
		);
	});

	it('rejects POST request without required fields', async () => {
		const response = await SELF.fetch('http://example.com/api/v1/contact/', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				Origin: 'http://localhost:5173'
			},
			body: JSON.stringify({})
		});

		expect(response.status).toBe(400);
	});

	it('accepts valid POST request with all required fields', async () => {
		fetchMock
			.get('https://test.kgdev.me')
			.intercept({ method: 'POST', path: '/turnstile/v0/siteverify' })
			.reply(200, JSON.stringify({ success: true }));

		fetchMock
			.get('https://api.cloudflare.com')
			.intercept({ method: 'POST', path: '/client/v4/accounts/test-account-id/email/sending/send' })
			.reply(
				200,
				JSON.stringify({
					success: true,
					errors: [],
					messages: [],
					result: { delivered: 'test@email.com' }
				})
			);

		const response = await SELF.fetch('http://example.com/api/v1/contact/', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				Origin: 'http://localhost:5173'
			},
			body: JSON.stringify({
				email: 'test@example.com',
				name: 'Test User',
				message: 'This is a test message',
				token: 'test-token'
			})
		});

		const data = await response.json();
		console.log('Response status:', response.status);
		console.log('Response data:', data);
		expect(response.status).toBe(200);
		expect(data).toStrictEqual({ success: true });
	});

	it('blocks requests from disallowed origins', async () => {
		const response = await SELF.fetch('http://example.com/api/v1/contact/', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				Origin: 'http://evil.com'
			},
			body: JSON.stringify({
				email: 'test@example.com',
				name: 'Test User',
				message: 'This is a test message',
				token: 'test-token'
			})
		});

		expect(response.status).toBe(403);
		expect(response.headers.get('Access-Control-Allow-Origin')).toBeNull();
	});

	it('rejects request when Turnstile validation fails', async () => {
		fetchMock
			.get('https://test.kgdev.me')
			.intercept({ method: 'POST', path: '/turnstile/v0/siteverify' })
			.reply(
				200,
				JSON.stringify({
					success: false,
					'error-codes': ['invalid-input-response']
				})
			);

		const response = await SELF.fetch('http://example.com/api/v1/contact/', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				Origin: 'http://localhost:5173'
			},
			body: JSON.stringify({
				email: 'test@example.com',
				name: 'Test User',
				message: 'This is a test message',
				token: 'invalid-token'
			})
		});

		expect(response.status).toBe(401);
	});
});

describe("WebSocket connect", () => {
  it("returns 426 when Upgrade header is absent", async () => {
    const response = await SELF.fetch("http://example.com/api/v1/connect/my-post", {
      headers: { Origin: "http://localhost:5173" },
    });

    expect(response.status).toBe(426);
  });

  it("returns 426 when Upgrade header is not 'websocket'", async () => {
    const response = await SELF.fetch("http://example.com/api/v1/connect/my-post", {
      headers: {
        Origin: "http://localhost:5173",
        Upgrade: "h2c",
      },
    });

    expect(response.status).toBe(426);
  });

  it("returns 403 when path is not allowed", async () => {
    const response = await SELF.fetch("http://example.com/api/v1/connect/secret-page", {
      headers: {
        Origin: "http://localhost:5173",
        Upgrade: "websocket",
      },
    });
    
    expect(response.status).toBe(403);
  });
});

	describe('Ask Question Worker', () => {
		it('rejects invalid post slugs before opening an SSE stream', async () => {
			const response = await SELF.fetch('http://example.com/api/v1/ask/BadSlug', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					Origin: 'http://localhost:5173'
				},
				body: JSON.stringify({
					question: 'What does this post say about retries?'
				})
			});

			const body = await response.json();

			expect(response.status).toBe(400);
			expect(response.headers.get('Content-Type')).toContain('application/json');
			expect(body).toMatchObject({ success: false });
		});

		it('returns a normal 404 when a post is allowed but not indexed', async () => {
			const response = await SELF.fetch('http://example.com/api/v1/ask/my-post', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					Origin: 'http://localhost:5173'
				},
				body: JSON.stringify({
					question: 'What does this post say about retries?'
				})
			});

			const body = await response.json();

			expect(response.status).toBe(404);
			expect(response.headers.get('Content-Type')).toContain('application/json');
			expect(body).toMatchObject({ success: false });
		});

		it('streams a fallback answer with a mocked AI embedding', async () => {
			await (env as any).BLOG_POST_QA_CACHE.put(
				'post_version:my-post',
			JSON.stringify({ v: 'test-version-1' })
		);

		vi.spyOn(env.AI, 'run').mockResolvedValue({
			data: [[0.0123, -0.0456, 0.0789, 0.1011]]
		} as any);

		fetchMock
			.get('https://api.cloudflare.com')
			.intercept({
				method: 'POST',
				path: '/client/v4/accounts/test-account-id/vectorize/v2/indexes/kgdev-me-blog/query'
			})
			.reply(
				200,
				JSON.stringify({
					success: true,
					result: {
						matches: [
							{
								score: 0.01,
								metadata: {
									chunk_id: 1,
									heading: 'Intro',
									text: 'A short excerpt that is not relevant enough to use.',
									post_slug: 'my-post',
									post_version: 'test-version-1'
								}
							}
						]
					}
				})
			);

		const response = await SELF.fetch('http://example.com/api/v1/ask/my-post', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				Origin: 'http://localhost:5173'
			},
			body: JSON.stringify({
				question: 'What does this post say about retries?'
			})
		});

		const body = await response.text();

		expect(response.status).toBe(200);
		expect(env.AI.run).toHaveBeenCalledWith(
			'test-embedding-model',
			expect.objectContaining({
				text: ['what does this post say about retries']
			})
		);
		expect(body).toContain('event: meta');
		expect(body).toContain('"cached":false');
		expect(body).toContain("I don't see that in this post.");
		expect(body).toContain('event: done');
	});
});
