import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import { normalizeAccession, parseRetrieveFasta } from "./ncbi-retrieve";

const fixture = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "../mock/retrieve-nm012131.fa"),
  "utf8",
);

describe("parseRetrieveFasta", () => {
  it("reads the NM_012131 FASTA fixture", () => {
    const result = parseRetrieveFasta(fixture, "NM_012131");
    assert.equal(result.accession, "NM_012131.3");
    assert.match(result.header, /CLDN17/);
    assert.equal(result.length, 1241);
    assert.match(result.sequence, /^>NM_012131\.3 /);
    assert.match(result.sequence, /TAAAA\n$/);
  });

  it("rejects NCBI-style error text", () => {
    assert.throws(
      () => parseRetrieveFasta("Error: Invalid uid NOT_A_GENE", "NOT_A_GENE"),
      /Accession not found: NOT_A_GENE/,
    );
  });

  it("rejects not-found responses", () => {
    assert.throws(
      () => parseRetrieveFasta("not found.", "NM_000000"),
      /Accession not found/,
    );
  });
});

describe("normalizeAccession", () => {
  it("trims surrounding whitespace", () => {
    assert.equal(normalizeAccession("  NM_012131.3  "), "NM_012131.3");
  });
});
