---
term: Durable Object
sources:
  - title: Cloudflare Durable Objects docs
    url: https://developers.cloudflare.com/durable-objects/
---

A Durable Object (DO) is a stateful compute primitive in Cloudflare Workers. Each DO is a single, uniquely identified instance that provides both execution and strongly consistent storage.

A DO is addressed by an ID. For any given ID, Cloudflare routes requests to the same logical instance, ensuring that only one instance is active at a time. This enables serialized execution: incoming requests are handled one at a time per object, giving you linearizable access to its state without external coordination mechanisms such as distributed locks.

Each DO has built-in persistent storage. This includes a transactional, strongly consistent storage layer (backed by SQLite) and a key-value interface, both scoped to the object. Storage operations are atomic within a request.

DOs are location-aware but not permanently pinned to a specific data centre. Cloudflare places and may relocate an object to optimize latency and load, while preserving the single-instance execution model.

DOs are instantiated on demand. When idle, they may be evicted from memory; subsequent requests re-instantiate the object and reload its state from persistent storage. Billing is based on execution and storage usage rather than idle time.

DOs support alarms, which allow an object to schedule a future invocation of its alarm() handler for deferred or background work
