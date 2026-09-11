import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import { defaultDesignInput, type DesignInput } from "./design-input";
import { toSidirectFormBody, toSpeValue } from "./sidirect-form";
import { extractSidirectTsv, parseSidirectHtml } from "./sidirect-parse";

const fixture = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "../mock/sidirect-cldn17.html"),
  "utf8",
);

const cldn17Input: DesignInput = {
  ...defaultDesignInput,
  accession: "NM_012131.3",
  sequence: `>NM_012131.3 Homo sapiens claudin 17 (CLDN17), mRNA
ATGCATTTACAACAGGTACTTCTAGTTAGGCCAAGTTCAGTCACAGCTACTGATTTGGACTAAAACGTTA
ATGGGCAGCAGCCAAGGAGAACATCATCAAAGACTTCTCTAGACTCAAAAGGCTTCCACGTTCTACATCTT
AGAGCATCTTCTACCACTCCGAATTGAACCAGTCTTCAAAGTAAAGGCAATGGCATTTTATCCCTTGCAAA
ATTGCTGGGCTGGTTCTTGGGTTCCTTGGCATGGTGGGGACTCTTGCCACAACCCTTCTGCCTCAGTGGAG
AGTATCAGCTTTTGTTGGCAGCAACATTATTGTCTTTGAGAGGCTCTGGGAAGGGCTCTGGATGAATTGC
ATCCGACAAGCCAGGGTCCGGTTGCAATGCAAGTTCTATAGCTCCTTGTTGGCTCTCCCGCCTGCCCTGG
AAAACAGCCCGGGCCCTCATGTGTGTGGCTGTTGCTCTCTCCTTGATCGCCCTGCTTATTGGCATCTGTGG
ACATGAAGCAGGTCCAGTGCACAGGCTCTAACGAGAGGGCCAAAGCATACCTTCTGGGAACTTCAGGAGTC
ACTCTTCATCCTGACGGGCATCTTCGTTCTGATTCCGGTGAGCTGGACAGCCAATATAATCATCAGAGATT
ATCTACAACCCAGCCATCCACATAGGTCAGAAACGAGAGCTGGGAGCAGCACTTTTCCTTGGCTGGGCAAG
ACGCTGCTGTCCTCTTCATTGGAGGGGGTCTGCTTTGTGGATTTTGCTGCTGCAACAGAAAGAAGCAAGGG
ATACAGATATCCAGTGCCTGGCTACCGTGTGCCACACACAGATAAGCGAAGAAATACGACAATGCTTAGTA
AGACCTCCACCAGTTATGTCTAATGCCTCCTTTTGGCTCCAAGTATGGACTATGGTCAATGTTTTTTATA
AAAGTCCTGCTAGAAACTGTAAGTATGTGAGGCAGGAGAACTTGCTTTATGTCTAGATTTACATTGATACG
AAAAGTTTCAATTTGTTACTGGTGGTAGGAATGAAAATGACTTACTTGGACATTCTGACTTCAGGTGTATT
AAAATGCATTGACTATTGTTGGACCCAATCGCTGCTCCAATTTTCATATTCTAAATTCAAGTATACCCATA
AATCATTAGCAAGTGTACAATGATGGACTACTTATTACTTTTTGACCATCATGTATTATCTGATAAGAATC
TAAAGTTGAAATTGATATTCTATAACAATAAAACATATACCTATTCTAAAA`,
  combine: "Ui-Tei × Reynolds × Amarzguioui",
  customPatternEnabled: true,
  customPattern: "NNGNNNNNNNNNNNNNNNNNNNN",
  gcMin: "0",
  gcMax: "100",
};

describe("extractSidirectTsv", () => {
  it("reads the export textarea", () => {
    const tsv = extractSidirectTsv(fixture);
    assert.ok(tsv);
    assert.match(tsv, /^\[siDirect v2\.1/);
    assert.match(tsv, /target position\ttarget sequence/);
  });
});

describe("parseSidirectHtml", () => {
  it("maps the CLDN17 fixture into SirnaResult", () => {
    const result = parseSidirectHtml(fixture, cldn17Input);

    assert.equal(result.transcript.id, "NM_012131.3");
    assert.equal(result.transcript.symbol, "CLDN17");
    assert.equal(result.transcript.name, "claudin 17");
    assert.equal(result.transcript.length, 1254);
    assert.equal(result.cds.start, 1);
    assert.equal(result.cds.end, 1254);
    assert.equal(result.sirnas.length, 10);

    const first = result.sirnas[0];
    assert.equal(first.id, "si-01");
    assert.equal(first.start, 45);
    assert.equal(first.end, 67);
    assert.equal(first.antisense, "UUUUAGUCCAAAUCAGUAGCU");
    assert.equal(first.sense, "CUACUGAUUUGGACUAAAACG");
    assert.deepEqual(first.rules, {
      "Ui-Tei": true,
      Reynolds: true,
      Amarzguioui: true,
    });
    assert.equal(first.seedTm, 20.3);
    assert.equal(first.offTarget?.minMismatchGuide, 2);
    assert.equal(first.offTarget?.passenger[3], 25);

    const withGt = result.sirnas.find((row) => row.start === 1096);
    assert.equal(withGt?.offTarget?.passenger[3], 100);
    assert.ok(result.sirnas.every((row) => row.score > 0));
  });

  it("falls back to the HTML table when the textarea is missing", () => {
    const withoutTsv = fixture.replace(/<textarea[\s\S]*<\/textarea>/, "");
    const result = parseSidirectHtml(withoutTsv, cldn17Input);
    assert.equal(result.sirnas.length, 10);
    assert.equal(result.sirnas[0]?.start, 45);
    assert.equal(result.sirnas[0]?.sense, "CUACUGAUUUGGACUAAAACG");
  });

  it("returns an empty candidate list instead of failing", () => {
    const empty = `
      <html><textarea>
      [siDirect v2.1]
      target position	target sequence	RNA oligo, guide	passenger	functional siRNA selection	seed-duplex stabilty (Tm), guide	passenger
      </textarea></html>
    `;
    const result = parseSidirectHtml(empty, cldn17Input);
    assert.deepEqual(result.sirnas, []);
  });
});

describe("toSidirectFormBody", () => {
  it("maps combine rules and specificity onto 2.1 field names", () => {
    assert.equal(toSpeValue("human-230"), "hs_refseq230");
    assert.equal(toSpeValue("mouse-225"), "mm_refseq225");
    assert.equal(toSpeValue("none"), "none");

    const union = toSidirectFormBody(defaultDesignInput);
    assert.equal(union.get("UorRorA"), "1");
    assert.equal(union.get("URA"), null);
    assert.equal(union.get("spe"), "hs_refseq230");
    assert.equal(union.get("uitei"), "1");
    assert.equal(union.get("hide"), "1");

    const intersection = toSidirectFormBody(cldn17Input);
    assert.equal(intersection.get("URA"), "1");
    assert.equal(intersection.get("UorRorA"), null);
    assert.equal(intersection.get("custom"), "1");
    assert.equal(intersection.get("customPattern"), "NNGNNNNNNNNNNNNNNNNNNNN");
  });
});
