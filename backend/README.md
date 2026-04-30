# Blog backend

A Cloudflare Worker for [kgdev.me](https://kgdev.me) that acts as the shared backend for blog features.

## What it does

- Handles contact form submissions.
- Validates Cloudflare Turnstile tokens before accepting contact requests.
- Sends contact messages through the Cloudflare Email Routing API.
- Hosts the live blog view-counter websocket endpoint backed by a Durable Object.

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

## Configuration

- `ALLOWED_ORIGINS` controls accepted browser origins.
- `ALLOWED_BLOG_PATHS` controls which blog slugs can use the realtime counter.
- `DESTINATION_EMAIL` sets the contact-form recipient.
- `CLOUDFLARE_TURNSTILE_SECRET_KEY`, `CLOUDFLARE_EMAIL_API_TOKEN`, and `CLOUDFLARE_ACCOUNT_ID` are required secrets.

## Stack

Written in Rust and compiled to WebAssembly using [worker-rs](https://github.com/cloudflare/workers-rs), with Cloudflare Durable Objects for realtime state.
