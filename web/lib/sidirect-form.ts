import {
  SPECIFICITY_OPTIONS,
  parseNucleotideSequence,
  type DesignInput,
} from "./design-input";
import type { DesignRuleName, DesignSettings } from "./sirna-types";

const COMBINE_FIELDS = {
  "Ui-Tei + Reynolds + Amarzguioui": "UorRorA",
  "Ui-Tei + Reynolds × Amarzguioui": "UorRA",
  "Ui-Tei × Reynolds × Amarzguioui": "URA",
} as const;

const SPECIES_PREFIX: Record<string, string> = {
  human: "hs",
  mouse: "mm",
  rat: "rn",
};

export function toSpeValue(specificity: string): string {
  if (specificity === "none") return "none";
  const [species, release] = specificity.split("-");
  const prefix = SPECIES_PREFIX[species];
  if (!prefix || !release) return specificity;
  return `${prefix}_refseq${release}`;
}

export function toSidirectFormBody(input: DesignInput): URLSearchParams {
  const params = new URLSearchParams();
  params.set("yourSeq", input.sequence);

  if (input.algorithms["Ui-Tei"]) params.set("uitei", "1");
  if (input.algorithms.Reynolds) params.set("reynolds", "1");
  if (input.algorithms.Amarzguioui) params.set("amarzguioui", "1");
  params.set(COMBINE_FIELDS[input.combine], "1");

  params.set("seedTm", "1");
  params.set("seedTmMax", String(input.seedTmMax));
  params.set("spe", toSpeValue(input.specificity));

  if (input.hideLessSpecific) params.set("hidenonspe", "1");
  if (input.showOffTargetHits) params.set("hitcount", "1");
  if (input.matchAllCriteria) params.set("hide", "1");

  if (input.targetRangeFrom.trim() && input.targetRangeTo.trim()) {
    params.set("pos", "1");
    params.set("posStart", input.targetRangeFrom.trim());
    params.set("posEnd", input.targetRangeTo.trim());
  } else {
    params.set("posStart", "start");
    params.set("posEnd", "end");
  }

  if (input.avoidContiguousGC) {
    params.set("consGC", "1");
    params.set("consGCmax", String(input.avoidContiguousGCMin));
  }
  if (input.avoidContiguousAT) {
    params.set("consAT", "1");
    params.set("consATmax", String(input.avoidContiguousATMin));
  }

  if (input.gcMin.trim() && input.gcMax.trim()) {
    params.set("percentGC", "1");
    params.set("percentGCMin", input.gcMin.trim());
    params.set("percentGCMax", input.gcMax.trim());
  }

  if (input.customPatternEnabled) {
    params.set("custom", "1");
    params.set("customPattern", input.customPattern);
  }
  if (input.excludePatternEnabled) {
    params.set("exclude", "1");
    params.set("excludePattern", input.excludePattern);
  }

  return params;
}

export function designSettingsFromInput(
  input: DesignInput,
  sequenceLength: number,
): DesignSettings {
  const algorithms = (
    ["Ui-Tei", "Reynolds", "Amarzguioui"] as DesignRuleName[]
  ).filter((name) => input.algorithms[name]);
  const { species, database } = specificityLabel(input.specificity);
  const from = Number(input.targetRangeFrom);
  const to = Number(input.targetRangeTo);

  return {
    length: 21,
    overhang: 2,
    algorithms,
    combine: input.combine,
    specificity: {
      species,
      database,
      hideLessSpecific: input.hideLessSpecific,
      showOffTargetHits: input.showOffTargetHits,
    },
    seedTmMax: input.seedTmMax,
    gcMin: input.gcMin.trim() === "" ? 0 : Number(input.gcMin),
    gcMax: input.gcMax.trim() === "" ? 100 : Number(input.gcMax),
    avoidContiguousGC: input.avoidContiguousGC ? input.avoidContiguousGCMin : 0,
    avoidContiguousAT: input.avoidContiguousAT ? input.avoidContiguousATMin : 0,
    targetRange: {
      from: Number.isInteger(from) && from > 0 ? from : 1,
      to: Number.isInteger(to) && to > 0 ? to : sequenceLength,
    },
  };
}

export function transcriptFromInput(input: DesignInput) {
  const parsed = parseNucleotideSequence(input.sequence);
  return {
    parsed,
    transcript: {
      id: accessionFromInput(input, parsed.header),
      symbol: symbolFromHeader(parsed.header, input.accession),
      name: nameFromHeader(parsed.header),
      length: parsed.length,
      sequence: parsed.sequence,
    },
  };
}

function specificityLabel(value: string): { species: string; database: string } {
  if (value === "none") return { species: "None", database: "None" };
  const option = SPECIFICITY_OPTIONS.find((item) => item.value === value);
  const label = option?.label ?? value;
  const [species, database] = label.split(" transcript, ");
  return {
    species: species ?? label,
    database: database ?? label,
  };
}

function accessionFromInput(input: DesignInput, header: string | null): string {
  const fromHeader = header?.match(
    /\b((?:NM|NR|XM|XR)_\d+(?:\.\d+)?)\b/i,
  )?.[1];
  if (fromHeader) return fromHeader;
  if (input.accession.trim()) return input.accession.trim();
  return "query";
}

function symbolFromHeader(header: string | null, accession: string): string {
  const symbol = header?.match(/\(([^)]+)\)\s*,?\s*(?:mRNA|transcript)?\s*$/i)?.[1];
  if (symbol) return symbol;
  if (accession.trim()) return accession.trim();
  return "QUERY";
}

function nameFromHeader(header: string | null): string {
  if (!header) return "query sequence";
  const named = header.match(
    /(?:sapiens|musculus|norvegicus)\s+(.+?)\s+\(/i,
  )?.[1];
  return named?.trim() || header;
}
