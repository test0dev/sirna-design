# sirna-design web

Next.js product UI (Design + Results). Independent frontend service; talks to the Rust API (`sirna-design` crate).

## Two-process dev

```bash
# terminal 1 — API (repo root)
cargo run -- --listen 127.0.0.1:8080

# terminal 2 — UI
cd web
npm install
npm run dev
```

Open [http://localhost:3000](http://localhost:3000).

| Variable | Default | Purpose |
|---|---|---|
| `NEXT_PUBLIC_API_BASE` | `http://127.0.0.1:8080` | Rust API origin used by `lib/api.ts` (`apiUrl`) |
| `CORS_ALLOW_ORIGIN` (API) | `http://localhost:3000`, `http://127.0.0.1:3000` | Browser origins allowed by axum |

Design submit is `POST ${NEXT_PUBLIC_API_BASE}/v1/design` with camelCase `DesignInput`. The response is a Rust `DesignResult` (same shape as `SirnaResult`); it is **not** wrapped as `{ result: ... }`.

Resolve and retrieve go to the Rust API: `GET /v1/resolve?symbol=…&include_sequence=1` and `GET /v1/retrieve?accession=…` (see `apiUrl`). They do not call siDirect, and they do not run a full design.

```bash
npm run typecheck
npm test
```
