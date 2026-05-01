---
term: Astro.js
sources:
  - title: Astro.js docs
    url: https://docs.astro.build/en/concepts/why-astro/
---

Astro (Astro.js) is a modern web framework for building content-focused websites with a strong emphasis on performance. Its core model is “server-first” rendering: components are rendered to static HTML at build time (or on the server), and JavaScript is only sent to the browser when explicitly needed. This “islands architecture” lets you add interactivity to isolated components (e.g., a search box or carousel) without hydrating the entire page, significantly reducing client-side JS.

Astro is framework-agnostic at the component level—you can use React, Vue.js, Svelte, or others within the same project. It supports static site generation by default, with optional server-side rendering for dynamic routes, and has built-in features for routing, content collections (useful for blogs/docs), and asset optimization. The result is a developer experience similar to component-based SPAs, but with the runtime characteristics of highly optimized static pages.
