# Blog backend

A Cloudflare Worker for [kgdev.me](https://kgdev.me) that acts as the shared backend for blog features.

## What it does

- Handles contact form submissions.
- Validates Cloudflare Turnstile tokens before accepting contact requests.
- Sends contact messages through the Cloudflare Email Routing API.
- Hosts the live blog view-counter websocket endpoint backed by a Durable Object.
- Answers questions about a single blog post via RAG (retrieval over post chunks in Vectorize, generation through Workers AI), streamed back as SSE.

## Contact flow

1. The frontend POSTs `{ name, email, message, token }` to `/api/v1/contact/`.
2. The worker validates the Turnstile token.
3. The message fields are validated.
4. A formatted HTML and plain-text email is sent to the configured destination address.
5. The worker responds with a JSON success or error payload.

## View-counter flow

1. The frontend opens a websocket connection to `/api/v1/connect/:page`.
2. The worker checks that the page slug is in `ALLOWED_BLOG_PATHS`.
3. The request is forwarded to the `VIEW_COUNTER` Durable Object.
4. The Durable Object tracks connected clients and persisted totals for that page.
5. Connected clients receive live count updates over the websocket.

## Ask-the-blog flow

1. The frontend POSTs `{ question }` to `/api/v1/ask/:page`; the response is `text/event-stream`.
2. The worker validates the slug and question, then checks the KV answer cache keyed by `{slug, post_version, question_hash}`.
3. On a cache miss the per-slug `BlogPostQA` Durable Object charges a token-bucket and per-day cap.
4. The worker embeds the question, queries Vectorize filtered to the post's current version, and streams the generation back as `meta` / `delta` / `done` SSE events.
5. The full answer is written to KV (30d TTL) so future identical questions short-circuit on the cached path.

Post chunks and `post_version:{slug}` keys are produced by the ingest job in `frontend/scripts/ingest/`.

## Configuration

- `ALLOWED_ORIGINS` controls accepted browser origins.
- `ALLOWED_BLOG_PATHS` controls which blog slugs can use the realtime counter and ask endpoint.
- `DESTINATION_EMAIL` sets the contact-form recipient.
- `CLOUDFLARE_TURNSTILE_SECRET_KEY`, `CLOUDFLARE_EMAIL_API_TOKEN`, and `CLOUDFLARE_ACCOUNT_ID` are required secrets.
- `CLOUDFLARE_VECTORIZE_API_TOKEN` is required for the ask-the-blog Vectorize REST queries.

## Stack

Written in Rust and compiled to WebAssembly using [worker-rs](https://github.com/cloudflare/workers-rs), with Cloudflare Durable Objects for realtime state.
