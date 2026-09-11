# sirna-design web

Next.js product UI (Design + Results). Independent frontend service; talks to the Rust API (`sirna-design` crate).

## Dev

```bash
# terminal 1 — API
cargo run -- --listen 127.0.0.1:8080

# terminal 2 — UI
cd web
npm install
npm run dev
```

Set `NEXT_PUBLIC_API_BASE` (default `http://127.0.0.1:8080`) to point at the Rust service.

## Status

Initial import from the local Next.js prototype. Rewiring off siDirect BFF onto Rust `/v1/design` is in progress.
