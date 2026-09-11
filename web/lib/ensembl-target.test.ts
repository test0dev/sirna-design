import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import {
  assembleResolvedTarget,
  pickManeRefSeq,
  pickRefSeqFromXrefs,
  selectCanonicalTranscript,
  stripVersion,
  type EnsemblGeneLookup,
  type EnsemblManeFeature,
  type EnsemblXref,
} from "./ensembl-target";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const lookup = JSON.parse(
  readFileSync(join(root, "mock/ensembl-pcsk9-lookup.json"), "utf8"),
) as EnsemblGeneLookup;
const xrefs = JSON.parse(
  readFileSync(join(root, "mock/ensembl-pcsk9-xrefs.json"), "utf8"),
) as EnsemblXref[];
const mane = JSON.parse(
  readFileSync(join(root, "mock/ensembl-pcsk9-mane.json"), "utf8"),
) as EnsemblManeFeature[];

describe("stripVersion", () => {
  it("removes trailing version suffixes", () => {
    assert.equal(stripVersion("ENST00000302118.5"), "ENST00000302118");
    assert.equal(stripVersion("ENST00000302118"), "ENST00000302118");
  });
});

describe("selectCanonicalTranscript", () => {
  it("prefers is_canonical for PCSK9", () => {
    const { transcript, method } = selectCanonicalTranscript(lookup);
    assert.equal(transcript.id, "ENST00000302118");
    assert.equal(method, "canonical");
  });

  it("falls back to canonical_transcript id match", () => {
    const clone: EnsemblGeneLookup = {
      ...lookup,
      Transcript: (lookup.Transcript ?? []).map((tx) => ({
        ...tx,
        is_canonical: 0,
      })),
    };
    const { transcript, method } = selectCanonicalTranscript(clone);
    assert.equal(transcript.id, "ENST00000302118");
    assert.equal(method, "canonical_transcript_match");
  });
});

describe("RefSeq / MANE picking", () => {
  it("reads MANE Select refseq_match", () => {
    assert.equal(pickManeRefSeq(mane), "NM_174936.4");
  });

  it("falls back to the first RefSeq_mRNA xref", () => {
    const picked = pickRefSeqFromXrefs(xrefs);
    assert.equal(picked.refseq, "NM_001407241.1");
  });
});

describe("assembleResolvedTarget", () => {
  it("builds the unified payload for PCSK9", () => {
    const { transcript, method } = selectCanonicalTranscript(lookup);
    const result = assembleResolvedTarget({
      gene: lookup,
      transcript,
      selectionMethod: method,
      refseq: pickManeRefSeq(mane),
      protein: null,
      maneSelect: true,
      cdsSequence: "A".repeat(2079),
    });

    assert.equal(result.gene.symbol, "PCSK9");
    assert.equal(result.gene.ensembl_gene_id, "ENSG00000169174");
    assert.equal(result.gene.strand, "+");
    assert.equal(result.gene.assembly, "GRCh38/hg38");
    assert.equal(
      result.mane_select_transcript.ensembl_transcript_stable,
      "ENST00000302118",
    );
    assert.equal(result.mane_select_transcript.ensembl_transcript, "ENST00000302118.5");
    assert.equal(result.mane_select_transcript.refseq, "NM_174936.4");
    assert.equal(result.mane_select_transcript.mane_select, true);
    assert.equal(result.mane_select_transcript.canonical, true);
    assert.equal(result.cds?.length, 2079);
    assert.ok(result.all_transcripts.length >= 1);
  });
});
