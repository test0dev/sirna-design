export type SelectionMethod =
  | "canonical"
  | "canonical_transcript_match"
  | "longest_protein_coding";

export interface ResolvedGene {
  symbol: string;
  ensembl_gene_id: string;
  gene_description: string;
  chromosome: string;
  strand: "+" | "-";
  gene_start: number;
  gene_end: number;
  assembly: string;
}

export interface ResolvedTranscript {
  ensembl_transcript: string;
  ensembl_transcript_stable: string;
  refseq: string | null;
  protein: string | null;
  biotype: string;
  canonical: boolean;
  selection_method: SelectionMethod;
  mane_select: boolean;
}

export interface ResolvedCdsMeta {
  length: number;
  genomic_start: number | null;
  genomic_end: number | null;
  sequence?: string;
}

export interface ResolvedTarget {
  gene: ResolvedGene;
  mane_select_transcript: ResolvedTranscript;
  cds?: ResolvedCdsMeta;
  all_transcripts: Array<{
    id: string;
    biotype: string;
    canonical: boolean;
  }>;
}

export interface EnsemblGeneLookup {
  id: string;
  display_name?: string;
  description?: string;
  seq_region_name?: string;
  start?: number;
  end?: number;
  strand?: number;
  assembly_name?: string;
  biotype?: string;
  canonical_transcript?: string;
  species?: string;
  Transcript?: EnsemblTranscript[];
}

export interface EnsemblTranscript {
  id: string;
  Parent?: string;
  display_name?: string;
  biotype?: string;
  is_canonical?: number | boolean;
  start?: number;
  end?: number;
  length?: number;
  strand?: number;
  version?: number;
  Translation?: {
    id?: string;
    start?: number;
    end?: number;
    length?: number;
  };
}

export interface EnsemblXref {
  dbname?: string;
  display_id?: string;
  primary_id?: string;
  info_type?: string;
  info_text?: string;
}

export interface EnsemblManeFeature {
  id?: string;
  type?: string;
  refseq_match?: string;
  feature_type?: string;
}

export interface EnsemblSequence {
  id?: string;
  seq?: string;
  molecule?: string;
}

const ENSEMBL_REST = "https://rest.ensembl.org";
const REQUEST_TIMEOUT_MS = 20_000;

export function stripVersion(id: string): string {
  return id.replace(/\.\d+$/, "");
}

export function normalizeGeneSymbol(symbol: string): string {
  return symbol.trim().toUpperCase();
}

export function selectCanonicalTranscript(
  gene: EnsemblGeneLookup,
): { transcript: EnsemblTranscript; method: SelectionMethod } {
  const transcripts = gene.Transcript ?? [];
  if (transcripts.length === 0) {
    throw new Error(`No transcripts returned for ${gene.display_name ?? gene.id}.`);
  }

  const canonical = transcripts.find(
    (tx) => tx.is_canonical === 1 || tx.is_canonical === true,
  );
  if (canonical) {
    return { transcript: canonical, method: "canonical" };
  }

  const wanted = gene.canonical_transcript
    ? stripVersion(gene.canonical_transcript)
    : null;
  if (wanted) {
    const matched = transcripts.find((tx) => stripVersion(tx.id) === wanted);
    if (matched) {
      return { transcript: matched, method: "canonical_transcript_match" };
    }
  }

  const coding = transcripts
    .filter((tx) => tx.biotype === "protein_coding")
    .sort((a, b) => (b.length ?? 0) - (a.length ?? 0));
  if (coding[0]) {
    return { transcript: coding[0], method: "longest_protein_coding" };
  }

  return {
    transcript: [...transcripts].sort(
      (a, b) => (b.length ?? 0) - (a.length ?? 0),
    )[0]!,
    method: "longest_protein_coding",
  };
}

export function pickRefSeqFromXrefs(xrefs: EnsemblXref[]): {
  refseq: string | null;
  protein: string | null;
} {
  const mrna = xrefs.filter((x) => x.dbname === "RefSeq_mRNA");
  const peptide = xrefs.find((x) => x.dbname === "RefSeq_peptide");

  const maneMention = mrna.find((x) =>
    /mane\s*select/i.test(`${x.info_text ?? ""} ${x.info_type ?? ""}`),
  );

  return {
    refseq: maneMention?.display_id ?? mrna[0]?.display_id ?? null,
    protein: peptide?.display_id ?? null,
  };
}

export function pickManeRefSeq(features: EnsemblManeFeature[]): string | null {
  const mane = features.find(
    (f) =>
      (f.type === "MANE_Select" || f.feature_type === "mane") &&
      Boolean(f.refseq_match),
  );
  return mane?.refseq_match ?? null;
}

export function assembleResolvedTarget(args: {
  gene: EnsemblGeneLookup;
  transcript: EnsemblTranscript;
  selectionMethod: SelectionMethod;
  refseq: string | null;
  protein: string | null;
  maneSelect: boolean;
  cdsSequence?: string | null;
}): ResolvedTarget {
  const { gene, transcript, selectionMethod, refseq, protein, maneSelect, cdsSequence } =
    args;
  const stable = stripVersion(transcript.id);
  const versioned =
    transcript.version != null
      ? `${stable}.${transcript.version}`
      : gene.canonical_transcript &&
          stripVersion(gene.canonical_transcript) === stable
        ? gene.canonical_transcript
        : stable;

  const description = (gene.description ?? "")
    .replace(/\s*\[Source:.*$/i, "")
    .trim();

  const cdsLength =
    cdsSequence?.length ??
    (transcript.Translation?.length != null
      ? transcript.Translation.length * 3 + 3
      : undefined);

  return {
    gene: {
      symbol: gene.display_name ?? gene.id,
      ensembl_gene_id: gene.id,
      gene_description: description || gene.display_name || gene.id,
      chromosome: gene.seq_region_name ?? "",
      strand: gene.strand === -1 ? "-" : "+",
      gene_start: gene.start ?? 0,
      gene_end: gene.end ?? 0,
      assembly:
        gene.assembly_name === "GRCh38" || !gene.assembly_name
          ? "GRCh38/hg38"
          : gene.assembly_name,
    },
    mane_select_transcript: {
      ensembl_transcript: versioned,
      ensembl_transcript_stable: stable,
      refseq,
      protein,
      biotype: transcript.biotype ?? "unknown",
      canonical:
        transcript.is_canonical === 1 ||
        transcript.is_canonical === true ||
        selectionMethod === "canonical" ||
        selectionMethod === "canonical_transcript_match",
      selection_method: selectionMethod,
      mane_select: maneSelect,
    },
    cds:
      cdsLength != null
        ? {
            length: cdsLength,
            genomic_start: transcript.Translation?.start ?? null,
            genomic_end: transcript.Translation?.end ?? null,
            ...(cdsSequence ? { sequence: cdsSequence } : {}),
          }
        : undefined,
    all_transcripts: (gene.Transcript ?? []).map((tx) => ({
      id: stripVersion(tx.id),
      biotype: tx.biotype ?? "unknown",
      canonical: tx.is_canonical === 1 || tx.is_canonical === true,
    })),
  };
}

async function ensemblGet<T>(path: string): Promise<T> {
  const response = await fetch(`${ENSEMBL_REST}${path}`, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });

  if (response.status === 404) {
    throw new EnsemblNotFoundError(`Ensembl resource not found: ${path}`);
  }
  if (response.status === 400 && path.includes("/lookup/symbol/")) {
    throw new EnsemblNotFoundError(`Ensembl resource not found: ${path}`);
  }
  if (!response.ok) {
    throw new Error(`Ensembl HTTP ${response.status} for ${path}`);
  }

  return (await response.json()) as T;
}

export class EnsemblNotFoundError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "EnsemblNotFoundError";
  }
}

export async function resolveManeSelect(
  symbol: string,
  species = "homo_sapiens",
  options: { includeCdsSequence?: boolean } = {},
): Promise<ResolvedTarget> {
  const normalized = normalizeGeneSymbol(symbol);
  if (!normalized) {
    throw new Error("Enter a gene symbol first.");
  }

  let gene: EnsemblGeneLookup;
  try {
    gene = await ensemblGet<EnsemblGeneLookup>(
      `/lookup/symbol/${encodeURIComponent(species)}/${encodeURIComponent(normalized)}?expand=1`,
    );
  } catch (error) {
    if (error instanceof EnsemblNotFoundError) {
      throw new EnsemblNotFoundError(`Gene not found: ${normalized}`);
    }
    throw error;
  }

  const { transcript, method } = selectCanonicalTranscript(gene);
  const stable = stripVersion(transcript.id);
  if (/\.\d+$/.test(stable)) {
    throw new Error(`Refusing versioned Ensembl ID: ${stable}`);
  }

  const [maneFeatures, xrefs] = await Promise.all([
    ensemblGet<EnsemblManeFeature[]>(
      `/overlap/id/${encodeURIComponent(stable)}?feature=mane`,
    ).catch(() => [] as EnsemblManeFeature[]),
    ensemblGet<EnsemblXref[]>(`/xrefs/id/${encodeURIComponent(stable)}`),
  ]);

  const fromXrefs = pickRefSeqFromXrefs(xrefs);
  const maneRefSeq = pickManeRefSeq(maneFeatures);
  const refseq = maneRefSeq ?? fromXrefs.refseq;
  if (!refseq) {
    throw new Error(
      `No RefSeq mRNA accession found for ${normalized} (${stable}).`,
    );
  }

  let cdsSequence: string | null = null;
  if (options.includeCdsSequence) {
    const cds = await ensemblGet<EnsemblSequence>(
      `/sequence/id/${encodeURIComponent(stable)}?type=cds`,
    );
    cdsSequence = cds.seq ?? null;
  }

  const assembly = gene.assembly_name ?? "GRCh38";

  return assembleResolvedTarget({
    gene: {
      ...gene,
      assembly_name: assembly === "GRCh38" ? "GRCh38" : assembly,
    },
    transcript,
    selectionMethod: method,
    refseq,
    protein: fromXrefs.protein,
    maneSelect: Boolean(maneRefSeq),
    cdsSequence,
  });
}
