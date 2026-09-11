import type { DesignRuleName } from "@/lib/sirna-types";

export const DESIGN_INPUT_STORAGE_KEY = "sirna-design-input";
export const DESIGN_RESULT_STORAGE_KEY = "sirna-design-result";
export const MAX_SEQUENCE_LENGTH = 10_000;
export const SAMPLE_ACCESSION = "NM_012131.3";
export const SAMPLE_GENE_SYMBOL = "CLDN17";

export type CombineRule =
  | "Ui-Tei + Reynolds + Amarzguioui"
  | "Ui-Tei + Reynolds × Amarzguioui"
  | "Ui-Tei × Reynolds × Amarzguioui";

export type ContiguousLength = 4 | 5 | 6 | 7;

export interface DesignInput {
  geneSymbol: string;
  accession: string;
  sequence: string;
  algorithms: Record<DesignRuleName, boolean>;
  combine: CombineRule;
  seedTmMax: number;
  specificity: string;
  hideLessSpecific: boolean;
  showOffTargetHits: boolean;
  targetRangeFrom: string;
  targetRangeTo: string;
  avoidContiguousGC: boolean;
  avoidContiguousGCMin: ContiguousLength;
  avoidContiguousAT: boolean;
  avoidContiguousATMin: ContiguousLength;
  gcMin: string;
  gcMax: string;
  customPatternEnabled: boolean;
  customPattern: string;
  excludePatternEnabled: boolean;
  excludePattern: string;
  matchAllCriteria: boolean;
}

export interface ParsedSequence {
  header: string | null;
  sequence: string;
  length: number;
  gcPercent: number | null;
  invalidChars: string[];
}

export const COMBINE_RULES: Array<{
  value: CombineRule;
  hint: string;
}> = [
  {
    value: "Ui-Tei + Reynolds + Amarzguioui",
    hint: "Union — keep candidates that pass any selected algorithm",
  },
  {
    value: "Ui-Tei + Reynolds × Amarzguioui",
    hint: "Keep candidates that pass (Ui-Tei or Reynolds) and Amarzguioui",
  },
  {
    value: "Ui-Tei × Reynolds × Amarzguioui",
    hint: "Intersection — keep candidates that pass every selected algorithm",
  },
];

export const SPECIFICITY_OPTIONS = [
  { value: "none", label: "None" },
  {
    value: "human-230",
    label: "Human (Homo sapiens) transcript, RefSeq release 230 (May, 2025)",
  },
  {
    value: "human-225",
    label: "Human (Homo sapiens) transcript, RefSeq release 225 (Jul, 2024)",
  },
  {
    value: "human-220",
    label: "Human (Homo sapiens) transcript, RefSeq release 220 (Sep, 2023)",
  },
  {
    value: "human-215",
    label: "Human (Homo sapiens) transcript, RefSeq release 215 (Nov, 2022)",
  },
  {
    value: "human-210",
    label: "Human (Homo sapiens) transcript, RefSeq release 210 (Jan, 2022)",
  },
  {
    value: "human-205",
    label: "Human (Homo sapiens) transcript, RefSeq release 205 (Mar, 2021)",
  },
  {
    value: "human-200",
    label: "Human (Homo sapiens) transcript, RefSeq release 200 (May, 2020)",
  },
  {
    value: "mouse-230",
    label: "Mouse (Mus musculus) transcript, RefSeq release 230 (May, 2025)",
  },
  {
    value: "mouse-225",
    label: "Mouse (Mus musculus) transcript, RefSeq release 225 (Jul, 2024)",
  },
  {
    value: "mouse-220",
    label: "Mouse (Mus musculus) transcript, RefSeq release 220 (Sep, 2023)",
  },
  {
    value: "mouse-215",
    label: "Mouse (Mus musculus) transcript, RefSeq release 215 (Nov, 2022)",
  },
  {
    value: "mouse-210",
    label: "Mouse (Mus musculus) transcript, RefSeq release 210 (Jan, 2022)",
  },
  {
    value: "mouse-205",
    label: "Mouse (Mus musculus) transcript, RefSeq release 205 (Mar, 2021)",
  },
  {
    value: "mouse-200",
    label: "Mouse (Mus musculus) transcript, RefSeq release 200 (May, 2020)",
  },
  {
    value: "rat-230",
    label: "Rat (Rattus norvegicus) transcript, RefSeq release 230 (May, 2025)",
  },
  {
    value: "rat-225",
    label: "Rat (Rattus norvegicus) transcript, RefSeq release 225 (Jul, 2024)",
  },
  {
    value: "rat-220",
    label: "Rat (Rattus norvegicus) transcript, RefSeq release 220 (Sep, 2023)",
  },
  {
    value: "rat-215",
    label: "Rat (Rattus norvegicus) transcript, RefSeq release 215 (Nov, 2022)",
  },
  {
    value: "rat-210",
    label: "Rat (Rattus norvegicus) transcript, RefSeq release 210 (Jan, 2022)",
  },
  {
    value: "rat-205",
    label: "Rat (Rattus norvegicus) transcript, RefSeq release 205 (Mar, 2021)",
  },
  {
    value: "rat-200",
    label: "Rat (Rattus norvegicus) transcript, RefSeq release 200 (May, 2020)",
  },
] as const;

export const ALGORITHM_META: Array<{
  key: DesignRuleName;
  short: string;
  citation: string;
  href: string;
  className: string;
}> = [
  {
    key: "Ui-Tei",
    short: "U",
    citation: "Nucleic Acids Res 32, 936–948 (2004)",
    href: "https://doi.org/10.1093/nar/gkh247",
    className: "bg-rose-50 text-rose-700",
  },
  {
    key: "Reynolds",
    short: "R",
    citation: "Nat Biotechnol 22, 326–330 (2004)",
    href: "https://doi.org/10.1038/nbt936",
    className: "bg-sky-50 text-sky-700",
  },
  {
    key: "Amarzguioui",
    short: "A",
    citation: "BBRC 316, 1050–1058 (2004)",
    href: "https://doi.org/10.1016/j.bbrc.2004.02.157",
    className: "bg-emerald-50 text-emerald-700",
  },
];

export function formatFasta(header: string, sequence: string, width = 60): string {
  const chunks = sequence.match(new RegExp(`.{1,${width}}`, "g")) ?? [sequence];
  return `>${header}\n${chunks.join("\n")}`;
}

export const SAMPLE_SEQUENCE = `>NM_012131.3 Homo sapiens claudin 17 (CLDN17), mRNA
ATGCATTTACAACAGGTACTTCTAGTTAGGCCAAGTTCAGTCACAGCTACTGATTTGGACTAAAACGTTA
TGGGCAGCAGCCAAGGAGAACATCATCAAAGACTTCTCTAGACTCAAAAGGCTTCCACGTTCTACATCTT
GAGCATCTTCTACCACTCCGAATTGAACCAGTCTTCAAAGTAAAGGCAATGGCATTTTATCCCTTGCAAA
TTGCTGGGCTGGTTCTTGGGTTCCTTGGCATGGTGGGGACTCTTGCCACAACCCTTCTGCCTCAGTGGAG
AGTATCAGCTTTTGTTGGCAGCAACATTATTGTCTTTGAGAGGCTCTGGGAAGGGCTCTGGATGAATTGC
ATCCGACAAGCCAGGGTCCGGTTGCAATGCAAGTTCTATAGCTCCTTGTTGGCTCTCCCGCCTGCCCTGG
AAACAGCCCGGGCCCTCATGTGTGTGGCTGTTGCTCTCTCCTTGATCGCCCTGCTTATTGGCATCTGTGG
CATGAAGCAGGTCCAGTGCACAGGCTCTAACGAGAGGGCCAAAGCATACCTTCTGGGAACTTCAGGAGTC
CTCTTCATCCTGACGGGCATCTTCGTTCTGATTCCGGTGAGCTGGACAGCCAATATAATCATCAGAGATT
TCTACAACCCAGCCATCCACATAGGTCAGAAACGAGAGCTGGGAGCAGCACTTTTCCTTGGCTGGGCAAG
CGCTGCTGTCCTCTTCATTGGAGGGGGTCTGCTTTGTGGATTTTGCTGCTGCAACAGAAAGAAGCAAGGG
TACAGATATCCAGTGCCTGGCTACCGTGTGCCACACACAGATAAGCGAAGAAATACGACAATGCTTAGTA
AGACCTCCACCAGTTATGTCTAATGCCTCCTTTTGGCTCCAAGTATGGACTATGGTCAATGTTTTTTATA
AAGTCCTGCTAGAAACTGTAAGTATGTGAGGCAGGAGAACTTGCTTTATGTCTAGATTTACATTGATACG
AAAGTTTCAATTTGTTACTGGTGGTAGGAATGAAAATGACTTACTTGGACATTCTGACTTCAGGTGTATT
AAATGCATTGACTATTGTTGGACCCAATCGCTGCTCCAATTTTCATATTCTAAATTCAAGTATACCCATA
ATCATTAGCAAGTGTACAATGATGGACTACTTATTACTTTTTGACCATCATGTATTATCTGATAAGAATC
TAAAGTTGAAATTGATATTCTATAACAATAAAACATATACCTATTCTAAAA
`;

export const defaultDesignInput: DesignInput = {
  geneSymbol: SAMPLE_GENE_SYMBOL,
  accession: SAMPLE_ACCESSION,
  sequence: SAMPLE_SEQUENCE,
  algorithms: {
    "Ui-Tei": true,
    Reynolds: true,
    Amarzguioui: true,
  },
  combine: "Ui-Tei + Reynolds + Amarzguioui",
  seedTmMax: 21.5,
  specificity: "human-230",
  hideLessSpecific: true,
  showOffTargetHits: true,
  targetRangeFrom: "",
  targetRangeTo: "",
  avoidContiguousGC: true,
  avoidContiguousGCMin: 4,
  avoidContiguousAT: true,
  avoidContiguousATMin: 4,
  gcMin: "30",
  gcMax: "52",
  customPatternEnabled: false,
  customPattern: "",
  excludePatternEnabled: false,
  excludePattern: "",
  matchAllCriteria: true,
};

const VALID_BASE = new Set(["A", "C", "G", "T", "U", "N"]);

export function parseNucleotideSequence(raw: string): ParsedSequence {
  const lines = raw.replace(/\r\n/g, "\n").split("\n");
  let header: string | null = null;
  const body: string[] = [];

  for (const line of lines) {
    if (line.startsWith(">") && header === null && body.length === 0) {
      header = line.slice(1).trim() || null;
      continue;
    }
    body.push(line);
  }

  const compact = body.join("").replace(/[\s0-9]/g, "").toUpperCase();
  const invalidChars = [
    ...new Set(compact.split("").filter((char) => !VALID_BASE.has(char))),
  ].sort();
  const sequence = compact
    .split("")
    .filter((char) => VALID_BASE.has(char))
    .join("");
  const gcCount = (sequence.match(/[GC]/g) ?? []).length;

  return {
    header,
    sequence,
    length: sequence.length,
    gcPercent: sequence.length === 0 ? null : (gcCount / sequence.length) * 100,
    invalidChars,
  };
}

export function validateDesignInput(input: DesignInput): string | null {
  const parsed = parseNucleotideSequence(input.sequence);

  if (parsed.length === 0) {
    return "Paste a nucleotide sequence, or retrieve one from an accession number.";
  }
  if (parsed.invalidChars.length > 0) {
    return `Sequence contains invalid characters: ${parsed.invalidChars.join(", ")}.`;
  }
  if (parsed.length > MAX_SEQUENCE_LENGTH) {
    return `Sequence is ${parsed.length.toLocaleString()} nt. The limit is ${MAX_SEQUENCE_LENGTH.toLocaleString()} nt.`;
  }
  if (!Object.values(input.algorithms).some(Boolean)) {
    return "Select at least one functional siRNA algorithm.";
  }
  if (!Number.isFinite(input.seedTmMax)) {
    return "Enter a numeric seed-duplex Tm ceiling.";
  }

  const from = input.targetRangeFrom.trim();
  const to = input.targetRangeTo.trim();
  if ((from && !to) || (!from && to)) {
    return "Enter both ends of the target range, or leave both blank.";
  }
  if (from && to) {
    const start = Number(from);
    const end = Number(to);
    if (!Number.isInteger(start) || !Number.isInteger(end)) {
      return "Target range must be integer coordinates.";
    }
    if (start < 1 || end < start || end > parsed.length) {
      return `Target range must fall within 1–${parsed.length}.`;
    }
  }

  const gcMin = input.gcMin.trim();
  const gcMax = input.gcMax.trim();
  if ((gcMin && !gcMax) || (!gcMin && gcMax)) {
    return "Enter both GC content limits, or leave both blank.";
  }
  if (gcMin && gcMax) {
    const min = Number(gcMin);
    const max = Number(gcMax);
    if (!Number.isFinite(min) || !Number.isFinite(max)) {
      return "GC content must be numeric percentages.";
    }
    if (min < 0 || max > 100 || min > max) {
      return "GC content must be between 0 and 100, with min ≤ max.";
    }
  }

  if (input.customPatternEnabled && !input.customPattern.trim()) {
    return "Enter a custom pattern, or turn the option off.";
  }
  if (input.excludePatternEnabled && !input.excludePattern.trim()) {
    return "Enter an exclude pattern, or turn the option off.";
  }

  return null;
}
