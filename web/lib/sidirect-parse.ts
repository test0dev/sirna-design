import type { DesignInput } from "./design-input";
import { designSettingsFromInput, transcriptFromInput } from "./sidirect-form";
import type {
  DesignRuleName,
  OffTargetHits,
  SirnaCandidate,
  SirnaResult,
} from "./sirna-types";

interface RawCandidate {
  start: number;
  end: number;
  target: string;
  guide: string;
  passenger: string;
  selection: string;
  guideTm: number;
  passengerTm: number;
  offTarget?: OffTargetHits;
}

const TEXTAREA_RE = /<textarea[^>]*>\n?([\s\S]*?)<\/textarea>/i;
const TABLE_ROW_RE =
  /<tr><td class=[wv]>(\d+-\d+)\s*<td class=[wv]>([ACGTN]+)\s*<td class=[wv]>([ACGU]+)<br>([ACGU]+)[\s\S]*?<td class=[wo]>([\s\S]*?)<td class=[wp]>[\s\S]*?<td class=[wp]>(-?[\d.]+)\s*&deg;C\s*<td class=[wp]>(-?[\d.]+)\s*&deg;C/gi;

export function parseSidirectHtml(html: string, input: DesignInput): SirnaResult {
  const { parsed, transcript } = transcriptFromInput(input);
  if (parsed.length === 0) {
    throw new Error("Design input is missing a nucleotide sequence.");
  }

  const raw =
    parseTextareaCandidates(html) ?? parseTableCandidates(html);
  if (!raw) {
    throw new Error("siDirect HTML did not contain a result table or TSV list.");
  }

  const sirnas = raw.map((candidate, index) =>
    toSirnaCandidate(candidate, index),
  );

  return {
    transcript,
    cds: { start: 1, end: transcript.length },
    design: designSettingsFromInput(input, transcript.length),
    sirnas,
  };
}

export function extractSidirectTsv(html: string): string | null {
  const match = html.match(TEXTAREA_RE);
  return match ? decodeHtml(match[1]).trim() : null;
}

function parseTextareaCandidates(html: string): RawCandidate[] | null {
  const tsv = extractSidirectTsv(html);
  if (tsv === null) return null;

  const rows: RawCandidate[] = [];
  for (const line of tsv.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("[") || trimmed.startsWith("#")) {
      continue;
    }
    if (/^target position\t/i.test(trimmed)) {
      continue;
    }

    const cols = trimmed.split("\t");
    if (cols.length < 7) continue;

    const range = parsePosition(cols[0] ?? "");
    if (!range) continue;

    rows.push({
      start: range.start,
      end: range.end,
      target: cols[1] ?? "",
      guide: cols[2] ?? "",
      passenger: cols[3] ?? "",
      selection: cols[4] ?? "",
      guideTm: parseNumber(cols[5]),
      passengerTm: parseNumber(cols[6]),
      offTarget: cols.length >= 17 ? parseOffTarget(cols) : undefined,
    });
  }

  return rows;
}

function parseTableCandidates(html: string): RawCandidate[] | null {
  const rows: RawCandidate[] = [];
  for (const match of html.matchAll(TABLE_ROW_RE)) {
    const range = parsePosition(match[1] ?? "");
    if (!range) continue;
    rows.push({
      start: range.start,
      end: range.end,
      target: match[2] ?? "",
      guide: match[3] ?? "",
      passenger: match[4] ?? "",
      selection: selectionFromFonts(match[5] ?? ""),
      guideTm: parseNumber(match[6]),
      passengerTm: parseNumber(match[7]),
    });
  }
  return rows.length > 0 ? rows : null;
}

function toSirnaCandidate(raw: RawCandidate, index: number): SirnaCandidate {
  const sense = rnaOligo(raw.passenger, raw.target);
  const antisense = rnaOligo(raw.guide, "");
  const seedTm = Math.max(raw.guideTm, raw.passengerTm);
  const rules = rulesFromSelection(raw.selection);

  return {
    id: `si-${String(index + 1).padStart(2, "0")}`,
    start: raw.start,
    end: raw.end,
    sense,
    antisense,
    gc: gcPercent(sense),
    seedTm,
    score: deriveScore(rules, seedTm),
    rules,
    offTarget: raw.offTarget,
  };
}

function parsePosition(value: string): { start: number; end: number } | null {
  const range = value.trim().match(/^(\d+)\s*-\s*(\d+)$/);
  if (range) {
    return { start: Number(range[1]), end: Number(range[2]) };
  }
  const start = value.trim().match(/^(\d+)$/);
  if (!start) return null;
  const from = Number(start[1]);
  return { start: from, end: from + 22 };
}

function rulesFromSelection(value: string): Record<DesignRuleName, boolean> {
  const token = value.replace(/<[^>]+>/g, "").toUpperCase();
  return {
    "Ui-Tei": token.includes("U"),
    Reynolds: token.includes("R"),
    Amarzguioui: token.includes("A"),
  };
}

function selectionFromFonts(html: string): string {
  return [
    /class=u>\s*U/i.test(html) ? "U" : "",
    /class=r>\s*R/i.test(html) ? "R" : "",
    /class=a>\s*A/i.test(html) ? "A" : "",
  ].join("");
}

function rnaOligo(preferred: string, fallback: string): string {
  const rna = preferred.replace(/T/g, "U").toUpperCase();
  if (rna.length > 0) return rna;
  return fallback.replace(/T/g, "U").toUpperCase();
}

function gcPercent(sequence: string): number {
  if (sequence.length === 0) return 0;
  const gc = (sequence.match(/[GC]/g) ?? []).length;
  return Number(((gc / sequence.length) * 100).toFixed(1));
}

function deriveScore(
  rules: Record<DesignRuleName, boolean>,
  seedTm: number,
): number {
  const passed = Object.values(rules).filter(Boolean).length;
  const tmScore = Math.max(0, 1 - seedTm / 30);
  return Number((0.7 * (passed / 3) + 0.3 * tmScore).toFixed(2));
}

function parseOffTarget(cols: string[]): OffTargetHits {
  return {
    minMismatchGuide: parseHit(cols[7]),
    minMismatchPassenger: parseHit(cols[8]),
    guide: [
      parseHit(cols[9]),
      parseHit(cols[10]),
      parseHit(cols[11]),
      parseHit(cols[12]),
    ],
    passenger: [
      parseHit(cols[13]),
      parseHit(cols[14]),
      parseHit(cols[15]),
      parseHit(cols[16]),
    ],
  };
}

function parseHit(value: string | undefined): number {
  if (!value) return 0;
  const normalized = value.replace(/^>/, "").trim();
  const parsed = Number(normalized);
  return Number.isFinite(parsed) ? parsed : 0;
}

function parseNumber(value: string | undefined): number {
  const parsed = Number(value?.trim());
  return Number.isFinite(parsed) ? parsed : 0;
}

function decodeHtml(value: string): string {
  return value
    .replace(/&gt;/g, ">")
    .replace(/&lt;/g, "<")
    .replace(/&amp;/g, "&")
    .replace(/&quot;/g, '"');
}
