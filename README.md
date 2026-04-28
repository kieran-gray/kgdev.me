# kgdev.me

Source for [kgdev.me](https://kgdev.me) — a personal blog and portfolio. Everything runs on Cloudflare: the site is a static Astro build served via Workers, and the backend features are small Rust workers compiled to WebAssembly.

## Projects

**[web](./web)** — The Astro blog. Static site generation.

**[services/contact-service](./services/contact-service)** — Handles contact form submissions. Validates Cloudflare Turnstile tokens and forwards messages via the Cloudflare Email Routing API.

**[services/view-counter](./services/view-counter)** — Tracks live view counts on blog posts using Cloudflare Durable Objects.

## Workspaces

This is a monorepo with an npm workspace (for the JS/TS tooling across all projects) and a Cargo workspace (for the shared Rust build profile and unified `Cargo.lock`).

Running `npm install` at the root installs dependencies for all projects. Each project retains its own scripts and can be worked on independently.
