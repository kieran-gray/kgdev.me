# KGDEV.me — Personal Blog

This is my personal blog. The stack is straightforward: Astro for the site generator and Tailwind CSS for styling. Deployed in CI/CD to Cloudflare pages.

## Features

- Astro v5 with static site output
- Tailwind CSS with Typography plugin
- Light/Dark mode toggle with persistent state
- Color scheme selector using CSS variables
- Markdown content with code syntax highlighting (rehype-pretty-code)
- SEO basics: RSS feed and sitemap

## Integrations

- @astrojs/tailwind — https://docs.astro.build/en/guides/integrations-guide/tailwind/
- @astrojs/react — https://docs.astro.build/en/guides/integrations-guide/react/
- @astrojs/sitemap — https://docs.astro.build/en/guides/integrations-guide/sitemap/
- @astrojs/rss — https://docs.astro.build/en/guides/rss/
- rehype-pretty-code — https://rehype-pretty-code.netlify.app/

## Project Structure

```
/
├── public/
├── src/
│   └── pages/
│       └── index.astro
└── package.json
```

- Pages live in `src/pages/` as `.astro` or `.md` files and map to routes by filename.
- Components are in `src/components/`.
- Static assets (images, etc.) go in `public/`.

## Developing

All commands run from the project root:

| Command                | Action                                           |
| :--------------------- | :----------------------------------------------- |
| `npm install`          | Install dependencies                             |
| `npm run dev`          | Start local dev server at `localhost:4321`       |
| `npm run build`        | Build the production site to `./dist/`           |
| `npm run preview`      | Preview the production build locally             |
| `npm run astro ...`    | Run CLI commands like `astro add`, `astro check` |
| `npm run astro --help` | Get help using the Astro CLI                     |
