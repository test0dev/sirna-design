import { NextResponse } from "next/server";
import {
  EnsemblNotFoundError,
  normalizeGeneSymbol,
  resolveManeSelect,
} from "@/lib/ensembl-target";
import { fetchSidirectFasta } from "@/lib/sidirect-retrieve";

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
    if (!refseq) {
      return NextResponse.json(
        { error: `No RefSeq mRNA accession found for ${symbol}.` },
        { status: 502 },
      );
    }

    if (!fetchSequence) {
      return NextResponse.json(resolved);
    }

    const fasta = await fetchSidirectFasta(refseq);
    return NextResponse.json({
      ...resolved,
      accession: fasta.accession,
      header: fasta.header,
      sequence: fasta.sequence,
      length: fasta.length,
    });
  } catch (error) {
    if (error instanceof EnsemblNotFoundError) {
      return NextResponse.json({ error: error.message }, { status: 404 });
    }
    const message =
      error instanceof Error ? error.message : "Failed to resolve target gene.";
    const notFound =
      message.startsWith("Gene not found") ||
      message.startsWith("Accession not found");
    return NextResponse.json(
      { error: message },
      { status: notFound ? 404 : 502 },
    );
  }
}
