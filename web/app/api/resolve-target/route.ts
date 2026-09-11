import { NextResponse } from "next/server";
import { formatFasta } from "@/lib/design-input";
import {
  EnsemblNotFoundError,
  fetchEnsemblSequence,
  normalizeGeneSymbol,
  resolveManeSelect,
} from "@/lib/ensembl-target";

export async function GET(request: Request) {
  const url = new URL(request.url);
  const symbol = normalizeGeneSymbol(url.searchParams.get("symbol") ?? "");
  const species = (url.searchParams.get("species") ?? "homo_sapiens").trim();
  const fetchSequence = url.searchParams.get("fetchSequence") !== "0";

  if (!symbol) {
    return NextResponse.json(
      { error: "Enter a gene symbol first." },
      { status: 400 },
    );
  }

  try {
    const resolved = await resolveManeSelect(symbol, species || "homo_sapiens", {
      includeCdsSequence: false,
    });

    const refseq = resolved.mane_select_transcript.refseq;
    const ensemblId = resolved.mane_select_transcript.ensembl_transcript_stable;
    if (!refseq && !ensemblId) {
      return NextResponse.json(
        { error: `No transcript accession found for ${symbol}.` },
        { status: 502 },
      );
    }

    if (!fetchSequence) {
      return NextResponse.json(resolved);
    }

    const cdna = await fetchEnsemblSequence(ensemblId, "cdna");
    const accession = refseq ?? ensemblId;
    const header = `${accession} ${resolved.gene.gene_description} (${resolved.gene.symbol}), mRNA`;
    return NextResponse.json({
      ...resolved,
      accession,
      header,
      sequence: formatFasta(header, cdna),
      length: cdna.length,
    });
  } catch (error) {
    if (error instanceof EnsemblNotFoundError) {
      return NextResponse.json({ error: error.message }, { status: 404 });
    }
    const message =
      error instanceof Error ? error.message : "Failed to resolve target gene.";
    const notFound = message.startsWith("Gene not found");
    return NextResponse.json(
      { error: message },
      { status: notFound ? 404 : 502 },
    );
  }
}
