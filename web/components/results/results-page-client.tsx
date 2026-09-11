"use client";

import { useEffect, useState } from "react";
import { useI18n } from "@/components/i18n-provider";
import { PageShell } from "@/components/page-shell";
import { ResultsView } from "@/components/results/results-view";
import { DESIGN_RESULT_STORAGE_KEY } from "@/lib/design-input";
import { resultData } from "@/lib/result-data";
import type { SirnaResult } from "@/lib/sirna-types";

export function ResultsPageClient() {
  const { t } = useI18n();
  const [result, setResult] = useState<SirnaResult | null>(null);

  useEffect(() => {
    const stored = window.sessionStorage.getItem(DESIGN_RESULT_STORAGE_KEY);
    if (!stored) {
      setResult(resultData);
      return;
    }

    try {
      setResult(JSON.parse(stored) as SirnaResult);
    } catch {
      setResult(resultData);
    }
  }, []);

  if (!result) {
    return (
      <PageShell>
        <p className="py-16 text-sm text-slate-500">{t("loadingResults")}</p>
      </PageShell>
    );
  }

  return (
    <PageShell>
      <ResultsView result={result} />
    </PageShell>
  );
}
