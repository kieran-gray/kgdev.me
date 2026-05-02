# rag-admin

Local-only Leptos app for ingesting blog posts and glossary entries into
Cloudflare Vectorize. Replaces the `frontend/scripts/ingest` Node script.

Not deployed. Run local only, point it at the locally running blog (or
production), paste in a Cloudflare token with Workers AI + Vectorize edit + KV
write permissions, and trigger ingestions from the UI with live logs.

## Run

```sh
cargo install cargo-leptos
cargo leptos watch --project rag-admin
```

Then open `http://127.0.0.1:3000`.

## Configuration

Settings persist to `rag-admin/data/settings.toml`. Initial values can be overlaid from
env vars (`BLOG_URL`, `VECTORIZE_INDEX_NAME`, `EMBEDDING_MODEL`,
`CLOUDFLARE_ACCOUNT_ID`, `CLOUDFLARE_RAG_INGEST_API_TOKEN`,
`BLOG_POST_QA_CACHE_KV_NAMESPACE_ID`) or set on the Settings page.

The ingestion manifest is written to `rag-admin/data/manifest.json` (separate from the
JS script's manifest).

## Architecture

Clean architecture:

- `src/server/domain/` — entities (BlogPost, Chunk, Manifest, etc.)
- `src/server/application/ports/` — trait abstractions over Cloudflare
- `src/server/application/` — chunker, ingest orchestration, job/log plumbing
- `src/server/infrastructure/` — reqwest HTTP client, Cloudflare REST adapters,
  HTTP-backed `BlogSource`, file-backed `ManifestStore`
- `src/server/api/` — SSE endpoint for streaming ingest logs
- `src/server/setup/` — `Config`, `AppState`
- `src/server_fns.rs` — Leptos server functions (the only client/server seam)
- `src/components/`, `src/pages/` — UI

The Cloudflare adapters (`WorkersAiEmbedder`, `CloudflareVectorStore`,
`CloudflareKvStore`) sit behind ports so they could be lifted into a shared
crate later and reused by the backend worker.
