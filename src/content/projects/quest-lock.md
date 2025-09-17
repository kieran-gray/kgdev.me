---
name: Quest-Lock
summary: Quest-Lock is a tool for digital detoxes that locks users out of their social media by splitting their password into shares that can only be recovered by completing real world quests.
website: https://quest-lock.com
status: active
pinned: true
tags: ['saas', 'security', 'encryption', 'maps']
tech:
  languages: ['Rust', 'TypeScript', 'SQL']
  frameworks: ['Axum', 'React', 'Vite', 'SQLx']
repos:
  - name: quest-lock
    url: https://github.com/kieran-gray/quest-lock
    role: monorepo
    private: true
components:
  - name: Backend API
    type: backend
    language: Rust
    framework: Axum
    hosting:
      provider: GCP
      service: Cloud Run
    notes:
      - PostgreSQL with SQLx; Auth0 JWT validation
  - name: Frontend App
    type: frontend
    language: TypeScript
    framework: React (Vite)
    hosting:
      provider: Cloudflare
      service: Pages
      url: https://quest-lock.com
    notes:
      - Mapbox GL, client-side encryption, TanStack Query
  - name: Database
    type: db
    language: SQL
    framework: PostgreSQL
    hosting:
      provider: GCP
      service: Cloud SQL
insights:
  performance:
    image: /images/projects/quest-lock-load-performance.webp
    notes:
      - Static assets served from Cloudflare Pages with global edge caching; fast TTFB and repeat visits.
      - React app built with Vite; optimized bundles and route-based code splitting keep the initial payload lean.
      - Not a separate marketing page, the landing page and app are all one app so a little harder to optimize.
  security:
    image: /images/projects/quest-lock-security.webp
    notes:
      - Zero‑knowledge model; client‑side encryption and Shamir’s Secret Sharing; server cannot reconstruct secrets.
      - Auth0 JWT validation on the API; all endpoints require verified tokens.
      - TLS enforced end‑to‑end.
---

Backend runs on Google Cloud Run with containerized deployment via GitHub Actions. The frontend is a Vite/React application with Auth0 authentication and Mapbox for geospatial quests.

#### Zero-Knowledge

The system is designed so that Quest-Lock never has access to a user's new password. This is achieved through client-side encryption and a secret-sharing scheme.

1. Password Generation & Splitting:
   - A user plans `n` quests and generates a new, secure password on the client.
   - This password is then split into `2n` cryptographic shares using Shamir's Secret Sharing.
2. Share Distribution & Storage:
   - All `2n` shares are encrypted client-side before any transmission.
   - The user's device stores `n` of the encrypted shares locally.
   - The remaining `n` shares are stored on the server, with each share being locked behind one of the `n` quests.
3. Reconstruction Threshold:
   - To reconstruct the password, a threshold of `k` shares is required.
   - The system enforces that `k` is always greater than `n` (`k > n`).

This model ensures that neither the user (with only `n` shares) nor the server (with only `n` shares) possesses enough information to reconstruct the password independently. Access requires the user to complete a sufficient number of quests to combine their local shares with the server-held shares, meeting the `k` threshold.
