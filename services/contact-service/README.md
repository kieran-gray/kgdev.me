# contact-service

A Cloudflare Worker that handles contact form submissions from [kgdev.me](https://kgdev.me).

## What it does

When a visitor submits the contact form, the worker validates the request against Cloudflare Turnstile to filter bots, then forwards the message as a formatted email via the Cloudflare Email Routing API.

## Request flow

1. The frontend POSTs `{ name, email, message, token }` to `/api/v1/contact/`
2. The worker verifies the Turnstile token
3. The message fields are validated (email format, length limits)
4. A formatted HTML and plain-text email is sent to the configured destination address
5. The worker responds `{ "success": true }` or `{ "success": false, "error": "..." }`

## Stack

Written in Rust and compiled to WebAssembly using [worker-rs](https://github.com/cloudflare/workers-rs).
