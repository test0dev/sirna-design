export type DesignRuleName = "Ui-Tei" | "Reynolds" | "Amarzguioui";

export interface Transcript {
  id: string;
  symbol: string;
  name: string;
  length: number;
  sequence: string;
}

export interface CoordinateRange {
  start: number;
  end: number;
}

export interface SpecificitySettings {
  species: string;
  database: string;
  hideLessSpecific: boolean;
  showOffTargetHits: boolean;
}

export interface DesignSettings {
  length: number;
  overhang: number;
  algorithms: DesignRuleName[];
  combine: string;
  specificity: SpecificitySettings;
  seedTmMax: number;
  gcMin: number;
  gcMax: number;
  avoidContiguousGC: number;
  avoidContiguousAT: number;
  targetRange: {
    from: number;
    to: number;
  };
}

export interface OffTargetHits {
  minMismatchGuide: number;
  minMismatchPassenger: number;
  guide: [number, number, number, number];
  passenger: [number, number, number, number];
}

export interface SirnaCandidate {
  id: string;
  start: number;
  end: number;
  sense: string;
  antisense: string;
  gc: number;
  seedTm: number;
  score: number;
  rules: Record<DesignRuleName, boolean>;
  offTarget?: OffTargetHits;
}

export interface SirnaResult {
  transcript: Transcript;
  cds: CoordinateRange;
  design: DesignSettings;
  sirnas: SirnaCandidate[];
}

export type TmBand = "under-10" | "under-15" | "under-21.5" | "functional";

export function getTmBand(seedTm: number): TmBand {
  if (seedTm < 10) return "under-10";
  if (seedTm < 15) return "under-15";
  if (seedTm < 21.5) return "under-21.5";
  return "functional";
}
