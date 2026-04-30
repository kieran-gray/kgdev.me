# kgdev.me

Source for [kgdev.me](https://kgdev.me) — a personal blog and portfolio. Everything runs on Cloudflare: the site is a static Astro build served via Workers, and the backend features are small Rust workers compiled to WebAssembly.

## Projects

**[frontend](./frontend)** — The Astro blog. Static site generation.

**[backend](./backend)** — Generic blog backend. Handles contact form submissions and realtime blog features such as the live view counter using Cloudflare Durable Objects.

## Workspaces

This is a monorepo with an npm workspace (for the JS/TS tooling across all projects) and a Cargo workspace (for the shared Rust build profile and unified `Cargo.lock`).

Running `npm install` at the root installs dependencies for all projects. Each project retains its own scripts and can be worked on independently.
