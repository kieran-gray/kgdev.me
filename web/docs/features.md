# Feature Flags

Feature flags are build-time only.

- `astro.config.ts` reads `wrangler.jsonc` and builds the feature registry at startup.
- `src/config/feature-flags.mjs` contains the env parsing logic shared by build tooling.
- `src/config/features.ts` is the typed runtime surface used inside the app.

## Flags

| Feature       | Disabled behavior                                                                                                        |
| ------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `search`      | No `<Search />`, no navbar trigger, no `data-pagefind-body`, no Pagefind postbuild step                                  |
| `viewCounter` | No `<ViewCounter />` mount and no view-counter client WS script                                                          |
| `contact`     | `/contact` route is not prerendered, footer link hidden, no Turnstile/contact script usage                               |
| `og`          | `/og/default.png` and `/og/[slug].png` not prerendered; `og:image` is omitted unless a fallback/static image is provided |
| `mermaid`     | `astro-mermaid` integration is not registered in `astro.config.ts`                                                       |
| `rss`         | No RSS `<link rel="alternate">`; `/rss.xml` route not prerendered                                                        |
| `projects`    | No `/projects` routes, no projects nav item, no homepage projects section, and tags only index posts                     |
| `books`       | No `/books` route and no books nav item                                                                                  |

## Source Of Truth

The same feature manifest is used by:

- `src/config/features.ts` for render-time gates inside pages/components
- `astro.config.ts` for route injection, integrations, and sitemap filtering
- `scripts/postbuild.mjs` for conditional Pagefind indexing

The values behind those gates come from `wrangler.jsonc`:

- Top-level `vars` are the production defaults used by `npm run build` and `wrangler deploy`.
- `env.dev.vars` powers `npm run dev` and `npm run preview`.
- Explicit `PUBLIC_*` process env vars override Wrangler config when needed.

## Wrangler

Wrangler is the source of truth for non-secret app config, including feature flags.

- Astro route injection happens at build/config time in `astro.config.ts`.
- The app solves the usual Astro/Wrangler split by reading `wrangler.jsonc` during the Astro build and injecting `PUBLIC_*` values into Vite.
- This means `npm run build`, `npm run dev`, and `wrangler deploy` all resolve from one config file instead of separate `.env` and CI variable sets.

Use top-level `vars` for production and `env.dev.vars` for local development. Use `.env` only for Wrangler system credentials such as `CLOUDFLARE_API_TOKEN`.

## Env Overrides

Set any of these to `false` to disable a feature at build time:

- `PUBLIC_FEATURE_SEARCH`
- `PUBLIC_FEATURE_VIEW_COUNTER`
- `PUBLIC_FEATURE_CONTACT`
- `PUBLIC_FEATURE_OG`
- `PUBLIC_FEATURE_MERMAID`
- `PUBLIC_FEATURE_RSS`
- `PUBLIC_FEATURE_PROJECTS`
- `PUBLIC_FEATURE_BOOKS`
