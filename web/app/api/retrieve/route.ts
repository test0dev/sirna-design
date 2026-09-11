import { NextResponse } from "next/server";
import { fetchNcbiFasta, normalizeAccession } from "@/lib/ncbi-retrieve";

export async function GET(request: Request) {
  const accession = normalizeAccession(
    new URL(request.url).searchParams.get("accession") ?? "",
  );
  if (!accession) {
    return NextResponse.json(
      { error: "Enter an accession number first." },
      { status: 400 },
    );
  }

  try {
    return NextResponse.json(await fetchNcbiFasta(accession));
  } catch (error) {
    const message =
      error instanceof Error ? error.message : "Failed to retrieve FASTA.";
    const notFound = message.startsWith("Accession not found");
    const badRequest = message === "Enter an accession number first.";
    return NextResponse.json(
      { error: message },
      { status: badRequest ? 400 : notFound ? 404 : 502 },
    );
  }
}
