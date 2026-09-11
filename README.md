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

## Migration order

1. `rules` + unit tests vs `testdata/pcsk9/pcsk9.atomic.json`
2. `tm` + unit tests vs atomic seed Tm fields
3. `design` + tests vs `pcsk9.design.default.json` / `pcsk9.design.alland.json`
4. Ensembl client
5. Remote offtarget client vs `pcsk9.offtarget-remote.json` shape
6. REST API + integration tests
