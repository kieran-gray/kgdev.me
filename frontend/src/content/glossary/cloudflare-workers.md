---
term: Cloudflare Workers
sources:
  - title: Cloudflare Workers docs
    url: https://developers.cloudflare.com/workers/
---

Cloudflare Workers is a serverless edge compute platform for running JavaScript, TypeScript, WebAssembly, and other supported languages on Cloudflare’s global network. Code executes in lightweight isolates close to the user, allowing requests to be handled with very low latency without provisioning or managing servers. Workers are event-driven (most commonly handling HTTP requests via the standard Fetch API) and are designed to be stateless by default, integrating with external services or platform features—such as Durable Objects, KV, R2, and D1—for persistence. The model emphasizes fast cold starts, horizontal scalability, and pay-for-use billing based on requests and compute time.
