import { parseNucleotideSequence } from "./design-input";

export interface RetrievedTranscript {
  accession: string;
  header: string;
  sequence: string;
  length: number;
}

const SIDIRECT_RETRIEVE_URL = "https://sidirect2.rnai.jp/retrieveFASTA.cgi";
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
    throw new Error(`siDirect did not return FASTA for ${requestedAccession}.`);
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

export async function fetchSidirectFasta(
  accession: string,
): Promise<RetrievedTranscript> {
  const normalized = normalizeAccession(accession);
  if (!normalized) {
    throw new Error("Enter an accession number first.");
  }

  let text: string;
  try {
    const upstream = await fetch(
      `${SIDIRECT_RETRIEVE_URL}?accession=${encodeURIComponent(normalized)}`,
      {
        headers: {
          Accept: "*/*",
          Referer: "https://sidirect2.rnai.jp/",
          "User-Agent":
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        },
        signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
      },
    );

    if (!upstream.ok) {
      throw new Error(`siDirect retrieve returned HTTP ${upstream.status}.`);
    }

    text = await upstream.text();
  } catch (error) {
    if (error instanceof Error && error.message.startsWith("siDirect retrieve")) {
      throw error;
    }
    if (error instanceof Error && error.name === "TimeoutError") {
      throw new Error("Sequence retrieval timed out.");
    }
    throw new Error("Could not reach siDirect retrieve.");
  }

  return parseRetrieveFasta(text, normalized);
}

function isRetrieveError(text: string): boolean {
  const compact = text.replace(/\s+/g, "").toLowerCase();
  return (
    SPREAD_ERROR_RE.test(text) ||
    compact.startsWith("error:") ||
    compact === "notfound." ||
    compact.includes("failedtounderstandid")
  );
}
