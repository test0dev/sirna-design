# sirna-design

Rust siRNA design engine. Migrated from a Node/Bun verification prototype.

## Scope (v1)

- Core algorithms: oligo derivation, GC / contiguous filters, Ui-Tei / Reynolds / Amarzguioui, combine rules, seed Tm (siDirect NN)
- Design pipeline over a transcript window
- Ensembl symbol → transcript resolve
- Remote offtarget client: `https://offtarget.0bot.dev`
- HTTP API (axum), redesigned REST (semantically equivalent to the prototype)

Out of scope for v1: shipping Node source, siDirect scrape/compare tooling, local transcriptome offtarget index.

## Golden fixtures

`testdata/pcsk9/` was generated once from the Node prototype (`specificity=none`) and is the regression baseline. Do not regenerate casually.

## Dev

```bash
cargo test
cargo run -- --listen 127.0.0.1:8080
```

Environment:

| Variable | Default | Purpose |
|---|---|---|
| `LISTEN_ADDR` | `0.0.0.0:8080` | Bind address (`--listen` overrides) |
| `OFFTARGET_URL` | `https://offtarget.0bot.dev` | Remote offtarget origin |
| `ENSEMBL_URL` | `https://rest.ensembl.org` | Ensembl REST origin |

## HTTP API

| Method | Path | Body | Response |
|---|---|---|---|
| `GET` | `/health` | — | `{ "status": "ok" }` |
| `GET` | `/v1/version` | — | `{ "name", "version" }` (crate) |
| `POST` | `/v1/design` | see below | `DesignResult` (same serde shape as `pcsk9.design.*.json`) |
| `POST` | `/v1/offtarget/check` | remote offtarget request | `{ "results": [...] }` |

`POST /v1/design` accepts either:

- `{ "symbol": "PCSK9", ...DesignInput overrides }` — resolve via Ensembl, then design
- `{ "sequence": "...", "accession": "...", "geneSymbol": "...", "cds": { "start", "end" }, ...overrides }` — design without Ensembl

DesignInput overrides use the existing camelCase field names (`specificity`, `hideLessSpecific`, `targetRangeFrom`, …).

`POST /v1/offtarget/check` is a proxy around the remote client (`OFFTARGET_URL`), same snake_case body as `https://offtarget.0bot.dev/v1/offtarget/check`.

## Migration order

1. `rules` + unit tests vs `testdata/pcsk9/pcsk9.atomic.json`
2. `tm` + unit tests vs atomic seed Tm fields
3. `design` + tests vs `pcsk9.design.default.json` / `pcsk9.design.alland.json`
4. Ensembl client
5. Remote offtarget client vs `pcsk9.offtarget-remote.json` shape
6. REST API + integration tests
