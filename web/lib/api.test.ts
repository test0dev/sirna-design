import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { apiBase, apiUrl } from "./api";
import { defaultDesignInput, toDesignPayload } from "./design-input";
import { asSirnaResult } from "./sirna-types";

describe("apiUrl", () => {
  it("joins the default local API origin with a path", () => {
    assert.equal(apiBase(), "http://127.0.0.1:8080");
    assert.equal(apiUrl("/v1/design"), "http://127.0.0.1:8080/v1/design");
    assert.equal(
      apiUrl("/v1/resolve?symbol=PCSK9&include_sequence=1"),
      "http://127.0.0.1:8080/v1/resolve?symbol=PCSK9&include_sequence=1",
    );
    assert.equal(apiUrl("health"), "http://127.0.0.1:8080/health");
  });
});

describe("toDesignPayload", () => {
  it("strips FASTA headers and sends numeric GC limits", () => {
    const payload = toDesignPayload(defaultDesignInput);
    assert.equal(payload.geneSymbol, "CLDN17");
    assert.equal(payload.accession, "NM_012131.3");
    assert.equal(typeof payload.sequence, "string");
    assert.match(payload.sequence as string, /^ATG/);
    assert.doesNotMatch(payload.sequence as string, />/);
    assert.equal(payload.gcMin, 30);
    assert.equal(payload.gcMax, 52);
    assert.equal(payload.seedTmMax, 21.5);
  });
});

describe("asSirnaResult", () => {
  it("accepts a Rust DesignResult object", () => {
    const result = asSirnaResult({
      transcript: {
        id: "NM_1",
        symbol: "X",
        name: "x",
        length: 23,
        sequence: "A".repeat(23),
      },
      cds: { start: 1, end: 23 },
      design: {
        length: 21,
        overhang: 2,
        algorithms: ["Ui-Tei"],
        combine: "Ui-Tei + Reynolds + Amarzguioui",
        specificity: {
          species: "Human (Homo sapiens)",
          database: "none",
          hideLessSpecific: false,
          showOffTargetHits: false,
        },
        seedTmMax: 21.5,
        gcMin: 30,
        gcMax: 52,
        avoidContiguousGC: 4,
        avoidContiguousAT: 4,
        targetRange: { from: 1, to: 23 },
      },
      sirnas: [],
    });
    assert.equal(result.transcript.symbol, "X");
    assert.equal(result.sirnas.length, 0);
  });

  it("rejects the old { result } wrapper", () => {
    assert.throws(
      () => asSirnaResult({ result: { transcript: {} } }),
      /missing transcript/,
    );
  });
});
