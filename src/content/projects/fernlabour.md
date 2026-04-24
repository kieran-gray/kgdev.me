---
name: Fern Labour
summary: FernLabour.com is a SaaS tool for labour tracking and real-time sharing. It combines a contraction timer with a private subscription system that lets loved ones follow along via SMS, WhatsApp, or email. The backend is an event-sourced system on Cloudflare Workers and Durable Objects, with one Durable Object per labour.
website: https://fernlabour.com
status: active
pinned: true
tags: ['saas', 'health', 'event-sourcing', 'cloudflare', 'cqrs']
tech:
  languages: ['Rust', 'TypeScript', 'SQL']
  frameworks: ['workers-rs', 'React', 'Vite', 'Next.js', 'Tailwind']
repos:
  - name: fern-labour-cloudflare
    url: https://github.com/kieran-gray/fern-labour-cloudflare
    role: monorepo
    private: false
components:
  - name: Marketing Site
    type: frontend
    language: TypeScript
    framework: Next.js
    github: https://github.com/kieran-gray/fern-labour-cloudflare/tree/main/apps/marketing
    hosting:
      provider: Cloudflare
      service: Pages
      url: https://fernlabour.com
    notes:
      - Static export for fast load times and easier hosting.
  - name: Labour Frontend
    type: frontend
    language: TypeScript
    framework: React (Vite)
    github: https://github.com/kieran-gray/fern-labour-cloudflare/tree/main/apps/labour/web
    hosting:
      provider: Cloudflare
      service: Pages
      url: https://app.fernlabour.com
    notes:
      - Contraction timer and labour tracking UI.
      - WebSocket connection to the labour's Durable Object for real-time updates; messages are used as React Query cache invalidation signals.
  - name: Admin Dashboard
    type: frontend
    language: TypeScript
    framework: React (Vite)
    github: https://github.com/kieran-gray/fern-labour-cloudflare/tree/main/apps/admin/web
    hosting:
      provider: Cloudflare
      service: Pages
    notes:
      - Used to review contact-us messages and outgoing notifications.
      - Cassette-futurism inspired styling.
  - name: API Worker
    type: worker
    language: Rust
    framework: workers-rs
    github: https://github.com/kieran-gray/fern-labour-cloudflare/tree/main/apps/labour/worker/src/api_worker
    hosting:
      provider: Cloudflare
      service: Workers
    notes:
      - The only publicly exposed surface of the labour backend.
      - Authenticates requests, extracts the labour id, and routes commands and per-labour queries to the matching Durable Object.
      - Cross-aggregate queries (e.g. a mother's labour history) are served from D1.
  - name: Labour Durable Object
    type: worker
    language: Rust
    framework: workers-rs
    github: https://github.com/kieran-gray/fern-labour-cloudflare/tree/main/apps/labour/worker/src/durable_object
    hosting:
      provider: Cloudflare
      service: Durable Objects
    notes:
      - One Durable Object per labour, each with its own embedded SQLite for the event store, sync read models, and the process manager's effect ledger.
      - Single-threaded execution gives serialised commands for free; the write path is fully synchronous after HTTP deserialisation.
      - An alarm fires immediately after each command to run sync projectors, broadcast events over hibernating WebSockets, run async projectors to D1, and dispatch policy effects.
  - name: Auth Service
    type: worker
    language: Rust
    framework: workers-rs
    github: https://github.com/kieran-gray/fern-labour-cloudflare/tree/main/services/auth-service
    hosting:
      provider: Cloudflare
      service: Workers
    notes:
      - Private worker reachable only via service bindings.
      - Validates tokens from Clerk (end users) and a Cloudflare-issued service token (internal callers).
  - name: User Service
    type: worker
    language: Rust
    framework: workers-rs
    github: https://github.com/kieran-gray/fern-labour-cloudflare/tree/main/services/user-service
    hosting:
      provider: Cloudflare
      service: Workers
    notes:
      - Wraps Clerk to return user details (name, email, phone).
  - name: Notification Service
    type: worker
    language: Rust
    framework: workers-rs
    github: https://github.com/kieran-gray/fern-labour-cloudflare/tree/main/services/notification-service
    hosting:
      provider: Cloudflare
      service: Workers + Durable Objects
    notes:
      - Reuses the Worker + DO + CQRS + event-sourcing pattern with one DO per notification.
      - Split into three workers -  Notification (aggregate DO), Generation (templates → HTML/SMS/WhatsApp bodies), and Dispatch (Resend for email, Twilio for SMS/WhatsApp, with delivery webhooks).
  - name: Contact Service
    type: worker
    language: Rust
    framework: workers-rs
    github: https://github.com/kieran-gray/fern-labour-cloudflare/tree/main/services/contact-service
    hosting:
      provider: Cloudflare
      service: Workers
    notes:
      - State-based worker that stores contact-us messages in D1 and pings me on Slack.
  - name: Shared Read Models
    type: db
    language: SQL
    framework: SQLite (D1)
    hosting:
      provider: Cloudflare
      service: D1
    notes:
      - Async projectors write cross-aggregate read models here (e.g. a mother's labour history).
      - Fine-grained per-labour data stays in the DO and is never projected out.
insights:
  performance:
    image: /images/projects/fernlabour-load-performance.webp
    notes:
      - Static Next.js marketing site and Vite/React app served via Cloudflare Pages with global edge caching.
      - Workers start in milliseconds in V8 isolates - no cold-start containers to keep warm.
      - Durable Objects run close to the first user of each labour and keep state in in-process SQLite, so reads and writes return in microseconds to low milliseconds.
      - WebSocket Hibernation keeps subscriber sockets open without billing for idle time.
  security:
    image: /images/projects/fernlabour-security.webp
    notes:
      - Authentication via Clerk (OIDC/JWT); the Auth service validates tokens and is only reachable over service bindings.
      - Only the API Worker is publicly exposed; all other workers are private and only reachable via service bindings.
      - All traffic over HTTPS; per-labour data is isolated inside its own Durable Object's SQLite.
      - Security headers locked down because the app only calls fernlabour/Cloudflare-internal domains.
---

```mermaid
flowchart LR
    Client(["Client\n(web / PWA)"])

    subgraph pub["Public"]
        API["API Worker"]
    end

    subgraph priv["Private Workers"]
        Auth["Auth Service"]
        User["User Service"]
        Notif["Notification Service\n(3 workers + DO)"]
        Contact["Contact Service"]
    end

    subgraph dos["Durable Objects"]
        LabourDO[("Labour DO\none per labour\nevent store · read models · ledger")]
    end

    D1[("D1\nshared read models")]

    Clerk[["Clerk"]]
    Resend[["Resend"]]
    Twilio[["Twilio"]]
    Slack[["Slack"]]

    Client -- "HTTPS" --> API
    Client == "WebSocket" ==> LabourDO

    API -- "service binding" --> Auth
    API -- "service binding" --> User
    API -- "command or\nper-labour query" --> LabourDO
    API -- "cross-aggregate\nquery" --> D1

    LabourDO -. "async projector" .-> D1
    LabourDO -- "service binding" --> Notif
    User -- "fetches data" --> Clerk
    Auth -- "verify JWT" --> Clerk
    Notif -- "email" --> Resend
    Notif -- "SMS / WhatsApp" --> Twilio
    Contact -. "writes" .-> D1
    Contact -- "alert" --> Slack
```

The backend is a full rewrite of the original Python/GCP stack onto Cloudflare Workers and Durable Objects in Rust. Each labour is its own Durable Object with its own SQLite event store, projections, and WebSocket subscribers.

Commands run synchronously on a single thread inside the DO, so appends never race, and an alarm fires immediately after the response to run sync projectors (DO-local SQLite), broadcast events to connected WebSockets, run async projectors (to D1, for cross-labour queries), and dispatch effects via a process manager.

Side effects (notifications, issuing follow-up commands, generating subscription tokens) go through a policy/effect ledger with per-effect idempotency keys, so alarm retries can't double-send.

See [Event sourcing on Cloudflare Workers and Durable Objects](/posts/event-sourcing-cloudflare) for a full walkthrough of the architecture, and the earlier [Fern Labour (legacy)](/projects/fernlabour-legacy) project for the original Python/GCP backend it replaced.
