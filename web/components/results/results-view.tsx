"use client";

import { useMemo, useState } from "react";
import { CandidateTable } from "@/components/results/candidate-table";
import { DesignParameters } from "@/components/results/design-parameters";
import { Summary } from "@/components/results/summary";
import { TranscriptMap } from "@/components/results/transcript-map";
import type { SirnaResult } from "@/lib/sirna-types";

export function ResultsView({ result }: { result: SirnaResult }) {
  const initialSelection = useMemo(
    () =>
      result.sirnas.reduce(
        (best, candidate) =>
          !best || candidate.score > best.score ? candidate : best,
        result.sirnas[0],
      )?.id ?? "",
    [result.sirnas],
  );
  const [selectedId, setSelectedId] = useState(initialSelection);

  return (
    <>
      <Summary result={result} />
      <DesignParameters design={result.design} />
      <div className="border-t border-slate-200">
        <CandidateTable
          candidates={result.sirnas}
          design={result.design}
          selectedId={selectedId}
          onSelect={setSelectedId}
        />
      </div>
      <div className="border-t border-slate-200">
        <TranscriptMap
          result={result}
          selectedId={selectedId}
          onSelect={setSelectedId}
        />
      </div>
    </>
  );
}
