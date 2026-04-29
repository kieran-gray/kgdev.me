# Feature Flags

Feature flags are build-time only and live in `src/config/features.mjs`.

## Flags

| Feature       | Disabled behavior                                                                                                        |
| ------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `search`      | No `<Search />`, no navbar trigger, no `data-pagefind-body`, no Pagefind postbuild step                                  |
| `viewCounter` | No `<ViewCounter />` mount and no view-counter client WS script                                                          |
| `contact`     | `/contact` route is not prerendered, footer link hidden, no Turnstile/contact script usage                               |
| `og`          | `/og/default.png` and `/og/[slug].png` not prerendered; `og:image` is omitted unless a fallback/static image is provided |
| `mermaid`     | `astro-mermaid` integration is not registered in `astro.config.mjs`                                                      |
| `rss`         | No RSS `<link rel="alternate">`; `/rss.xml` route not prerendered                                                        |

## Source Of Truth

The same feature manifest is used by:

- `src/config/site.config.ts` for render-time gates
- `astro.config.mjs` for integration and sitemap filtering
- `scripts/postbuild.mjs` for conditional Pagefind indexing

## Env Overrides

Set any of these to `false` to disable a feature at build time:

- `PUBLIC_FEATURE_SEARCH`
- `PUBLIC_FEATURE_VIEW_COUNTER`
- `PUBLIC_FEATURE_CONTACT`
- `PUBLIC_FEATURE_OG`
- `PUBLIC_FEATURE_MERMAID`
- `PUBLIC_FEATURE_RSS`
