import type { Metadata } from "next";
import { ResultsPageClient } from "@/components/results/results-page-client";

export const metadata: Metadata = {
  title: "siRNA Design Results",
};

export default function ResultsPage() {
  return <ResultsPageClient />;
}
