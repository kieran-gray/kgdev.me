# Setup

This document covers the files and commands required to configure and run the blog.

## Quick Start

Run the interactive setup script from the `web/` directory:

```bash
npm run setup
```

It will walk you through your site URL, author details, social links, feature flags, and homepage copy, then write the config files for you. After it finishes, add your posts to `src/content/posts/` and remove the example posts.

## Manual Configuration

If you prefer to edit files directly, the main places to change are:

1. [`blog.config.ts`](../blog.config.ts)
   Site URL, brand name, author details, navigation, social links, favicon, and default theme.
2. [`src/content/pages/home.md`](../src/content/pages/home.md)
   Homepage headline, role summary, and body copy.
3. [`src/content/posts/`](../src/content/posts/)
   Add your posts and remove the example posts.

Optional features are configured in [`wrangler.jsonc`](../wrangler.jsonc).

## Feature Flags

Use `PUBLIC_FEATURE_*` values in `wrangler.jsonc` to enable or disable optional features:

- `PUBLIC_FEATURE_SEARCH`
- `PUBLIC_FEATURE_VIEW_COUNTER`
- `PUBLIC_FEATURE_CONTACT`
- `PUBLIC_FEATURE_OG`
- `PUBLIC_FEATURE_MERMAID`
- `PUBLIC_FEATURE_RSS`
- `PUBLIC_FEATURE_PROJECTS`
- `PUBLIC_FEATURE_BOOKS`

If you disable a route-based feature such as `contact`, `projects`, or `books`, restart the dev server.

## Runtime Endpoints

These values are only needed when the matching feature is enabled:

- `PUBLIC_VIEW_COUNTER_URL`
- `PUBLIC_CONTACT_URL`
- `PUBLIC_TURNSTILE_SITE_KEY`

Set production values in top-level `vars`.
Set local development values in `env.dev.vars`.

## Local Development

From the repository root:

```bash
npm install
cd web
npm run dev
```

## Build

From `web/`:

```bash
npm run build
```

## Deploy

Authenticate Wrangler:

```bash
wrangler login
```

Then run:

```bash
npm run deploy
```

For CI deploys, set:

- `CLOUDFLARE_API_TOKEN`
- `CLOUDFLARE_ACCOUNT_ID`

## Books Import

If you enable books, convert a Goodreads export with:

```bash
npm run books:import
```

Default input:
`src/content/goodreads_library_export.csv`

Default output:
`src/features/books/content/books.json`

You can also pass custom paths:

```bash
node scripts/convert-goodreads-export.mjs /path/to/goodreads_library_export.csv src/features/books/content/books.json
```

## Verification

From `web/`:

```bash
npm run verify-template
```
