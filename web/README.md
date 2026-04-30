# KGDEV.me — Personal Blog

This is my personal blog. The stack is Astro for the site generator, Tailwind CSS for styling, and Cloudflare Workers for deployment.

## Features

- Astro v5 with static site output
- Tailwind CSS with Typography plugin
- Light/Dark mode toggle with persistent state
- Color scheme selector using CSS variables
- Markdown content with code syntax highlighting (rehype-pretty-code)
- SEO basics: RSS feed and sitemap

## Setup

Run the interactive setup script to configure the site for the first time:

```bash
npm run setup
```

It will ask about your site URL, author details, social links, features, and homepage copy, then write `blog.config.ts`, `wrangler.jsonc`, and `src/content/pages/home.md` for you.

Full setup instructions: [`docs/setup.md`](./docs/setup.md).
Feature flag details: [`docs/features.md`](./docs/features.md).

## Integrations

- @astrojs/tailwind — https://docs.astro.build/en/guides/integrations-guide/tailwind/
- @astrojs/sitemap — https://docs.astro.build/en/guides/integrations-guide/sitemap/
- @astrojs/rss — https://docs.astro.build/en/guides/rss/
- rehype-pretty-code — https://rehype-pretty-code.netlify.app/

## Project Structure

```
/
├── public/
├── src/
│   ├── config/
│   ├── content/
│   ├── features/
│   ├── layouts/
│   └── pages/
├── scripts/
└── package.json
```

- Core always-on pages live in `src/pages/`.
- Optional routes and UI live in `src/features/<name>/`.
- Shared primitives live in `src/components/`.
- Static assets go in `public/`.

## Developing

All commands run from the project root:

| Command                   | Action                                               |
| :------------------------ | :--------------------------------------------------- |
| `npm install`             | Install dependencies                                 |
| `npm run setup`           | Interactive first-time configuration                 |
| `npm run dev`             | Start the local dev server at `localhost:4321`       |
| `npm run build`           | Build the production site to `./dist/`               |
| `npm run preview`         | Build with local dev vars and preview via Wrangler   |
| `npm run books:import`    | Convert a Goodreads CSV export into `books.json`     |
| `npm run verify-template` | Run typecheck, build, and feature-flag matrix checks |
| `npm run astro ...`       | Run CLI commands like `astro add`, `astro check`     |
| `npm run astro --help`    | Get help using the Astro CLI                         |
