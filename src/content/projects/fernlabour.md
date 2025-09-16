---
name: Fern Labour
summary: FernLabour.com is a modern SaaS tool for labour tracking and real-time sharing. It combines a contraction timer with a private subscription system that lets loved ones follow along with updates via SMS, WhatsApp, or email.
website: https://fernlabour.com
status: active
tags: ['saas', 'health']
tech:
  languages: ['TypeScript', 'Python', 'SQL']
  frameworks: ['React', 'Next.js', 'FastAPI', 'SQLAlchemy']
repos:
  - name: fern-labour-monorepo
    url: https://github.com/kieran-gray/fern-labour
    role: monorepo
    private: false
  - name: fern-labour-pub-sub
    url: https://github.com/kieran-gray/fern-labour-pub-sub
    role: package
    private: false
  - name: fern-labour-core
    url: https://github.com/kieran-gray/fern-labour-core
    role: package
    private: false
  - name: fern-labour-notifications-shared
    url: https://github.com/kieran-gray/fern-labour-notifications-shared
    role: package
    private: false
components:
  - name: Marketing Site
    type: frontend
    language: TypeScript
    framework: Next.js
    github: https://github.com/kieran-gray/fern-labour/tree/main/marketing
    hosting:
      provider: Cloudflare
      service: Pages
      url: https://fernlabour.com
    notes:
      - Static export for faster load times, easier hosting, and better SEO.
  - name: Frontend App
    type: frontend
    language: TypeScript
    framework: React (Vite)
    github: https://github.com/kieran-gray/fern-labour/tree/main/frontend
    hosting:
      provider: Cloudflare
      service: Pages
      url: https://track.fernlabour.com
    notes:
      - Labour tracking application. Supports PWA installation and offline tracking.
  - name: Labour Service API
    type: backend
    language: Python
    framework: FastAPI
    github: https://github.com/kieran-gray/fern-labour/tree/main/labour_service
    hosting:
      provider: GCP
      service: Cloud Run
    notes:
      - Contraction analysis, subscriber relationships, Stripe
  - name: Notification Service
    type: backend
    language: Python
    framework: FastAPI
    github: https://github.com/kieran-gray/fern-labour/tree/main/notification_service
    hosting:
      provider: GCP
      service: Cloud Run
    notes:
      - SMS, WhatsApp, Email via Twilio/SMTP
  - name: Contact Service
    type: backend
    language: Python
    framework: FastAPI
    github: https://github.com/kieran-gray/fern-labour/tree/main/contact_service
    hosting:
      provider: GCP
      service: Cloud Run
    notes:
      - Slack integration and Cloudflare Turnstile
  - name: Auth
    type: infra
    language: N/A
    framework: Keycloak
    hosting:
      provider: GCP
      service: Compute Engine
    notes:
      - OIDC provider for platform
  - name: Database
    type: db
    language: SQL
    framework: PostgreSQL
    hosting:
      provider: GCP
      service: Cloud SQL
    notes:
      - Sensitive information is encrypted at rest.
---

See repository documentation for full architecture. Key stacks include FastAPI-based services, React/Vite frontend, and a static Next.js marketing site. Messaging uses Twilio and SMTP with templated notifications. Authentication via Keycloak. Local development orchestrated with Docker Compose. Event messaging uses Google Pub/Sub (emulated in dev).
