import { parseNucleotideSequence } from "./design-input";

export interface RetrievedTranscript {
  accession: string;
  header: string;
  sequence: string;
  length: number;
}

const NCBI_EFETCH_URL =
  "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi";
const REQUEST_TIMEOUT_MS = 15_000;
const SPREAD_ERROR_RE = /^error\s*:/i;

export function normalizeAccession(value: string): string {
  return value.trim();
}

export function parseRetrieveFasta(
  text: string,
  requestedAccession: string,
): RetrievedTranscript {
  const body = text.replace(/^\uFEFF/, "").trim();
  if (!body) {
    throw new Error(`No sequence returned for ${requestedAccession}.`);
  }
  if (isRetrieveError(body)) {
    throw new Error(`Accession not found: ${requestedAccession}.`);
  }
  if (!body.startsWith(">")) {
    throw new Error(`NCBI did not return FASTA for ${requestedAccession}.`);
  }

  const parsed = parseNucleotideSequence(body);
  if (parsed.length === 0) {
    throw new Error(`Retrieved FASTA for ${requestedAccession} has no bases.`);
  }

  return {
    accession:
      parsed.header?.match(/\b((?:NM|NR|XM|XR)_\d+(?:\.\d+)?)\b/i)?.[1] ??
      requestedAccession,
    header: parsed.header ?? requestedAccession,
    sequence: body.replace(/\r\n/g, "\n").trim() + "\n",
    length: parsed.length,
  };
}

export async function fetchNcbiFasta(
  accession: string,
): Promise<RetrievedTranscript> {
  const normalized = normalizeAccession(accession);
  if (!normalized) {
    throw new Error("Enter an accession number first.");
  }

  const url = new URL(NCBI_EFETCH_URL);
  url.searchParams.set("db", "nuccore");
  url.searchParams.set("id", normalized);
  url.searchParams.set("rettype", "fasta");
  url.searchParams.set("retmode", "text");
  url.searchParams.set("tool", "sirna-design");

  let text: string;
  try {
    const upstream = await fetch(url, {
      headers: { Accept: "text/plain" },
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    });

    if (!upstream.ok) {
      throw new Error(`NCBI retrieve returned HTTP ${upstream.status}.`);
    }

    text = await upstream.text();
  } catch (error) {
    if (error instanceof Error && error.message.startsWith("NCBI retrieve")) {
      throw error;
    }
    if (error instanceof Error && error.name === "TimeoutError") {
      throw new Error("Sequence retrieval timed out.");
    }
    throw new Error("Could not reach NCBI retrieve.");
  }

  return parseRetrieveFasta(text, normalized);
}

function isRetrieveError(text: string): boolean {
  const compact = text.replace(/\s+/g, "").toLowerCase();
  return (
    SPREAD_ERROR_RE.test(text) ||
    compact.startsWith("error:") ||
    compact.startsWith("<?xml") ||
    compact === "notfound." ||
    compact.includes("failedtounderstandid")
  );
}
