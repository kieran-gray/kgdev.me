---
name: KGDEV.me
summary: The blog that you are currently reading.
website: https://kgdev.me
status: active
tags: ['blog']
tech:
  languages: ['TypeScript']
  frameworks: ['Astro', 'Tailwind']
repos:
  - name: kgdev.me
    url: https://github.com/kieran-gray/kgdev.me
    role: repo
    private: false
components:
  - name: Website
    type: frontend
    language: TypeScript
    framework: Astro
    hosting:
      provider: Cloudflare
      service: Pages
      url: https://kgdev.me
    notes:
      - Static export for super fast load times.
insights:
  performance:
    image: /images/projects/kgdevme-load-performance.webp
    notes:
      - Astro static site; minimal client‑side JS for excellent core‑web‑vitals.
      - Served via Cloudflare Pages with edge caching; compressed assets and long‑lived cache headers.
      - Tailwind CSS JIT keeps styles lean; images are optimized and lazy‑loaded.
  security:
    image: /images/projects/kgdevme-security.webp
    notes:
      - Static hosting with no runtime server reduces attack surface significantly.
      - All traffic over HTTPS; Cloudflare Pages headers set via `_headers` file.
---

Based on the following template: https://github.com/nicdun/astro-tech-blog

Added colour schemes, Projects, CI/CD, and some personal preference styling updates.
