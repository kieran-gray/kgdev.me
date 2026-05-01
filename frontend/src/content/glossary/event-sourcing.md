---
term: Event Sourcing
sources:
  - title: Martin Fowler — Event Sourcing
    url: https://martinfowler.com/eaaDev/EventSourcing.html
---

Event sourcing is a persistence pattern in which the system of record is an append-only sequence of immutable domain events, rather than the latest snapshot of state. State is derived by replaying these events in order. For example, instead of mutating a record to set status = 'active', the system appends a LabourStarted event; the current state is the result of applying all events for that entity.

Events are written atomically to an ordered log (often per aggregate or stream). They are never updated or deleted; corrections are expressed as new events. Rebuilding state is deterministic: given the same event stream and reducer logic, you obtain the same result.

The approach provides a complete audit trail (every change is captured as a fact with context), supports temporal queries (“what was the state at time T?”), and allows read models to be rebuilt or evolved by replaying the log. It aligns well with domain-driven design, where events correspond to meaningful domain facts.

The trade-offs are operational and design complexity: managing event schema evolution and versioning, coordinating projection rebuilds, handling ordering and at-least-once delivery semantics in consumers, and ensuring idempotent event handling.

Event sourcing is commonly paired with CQRS (Command Query Responsibility Segregation): commands validate and append events to the write model, while queries read from one or more projections (read models) that are derived asynchronously from the event stream.
