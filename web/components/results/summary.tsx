"use client";

import { Dna, FlaskConical } from "lucide-react";
import Link from "next/link";
import { useI18n } from "@/components/i18n-provider";
import type { SirnaResult } from "@/lib/sirna-types";

interface SummaryProps {
  result: SirnaResult;
}

export function Summary({ result }: SummaryProps) {
  const { t } = useI18n();
  const { transcript, cds, sirnas } = result;
  const qualified = sirnas.filter(
    (candidate) =>
      candidate.seedTm <= result.design.seedTmMax &&
      candidate.gc >= result.design.gcMin &&
      candidate.gc <= result.design.gcMax &&
      Object.values(candidate.rules).every(Boolean),
  ).length;

  return (
    <header className="border-b border-slate-200 pb-6 sm:pb-7">
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.18em] text-indigo-600">
          <FlaskConical aria-hidden="true" className="size-4" />
          {t("brand")}
        </div>
        <Link
          href="/"
          className="text-xs text-slate-500 outline-none hover:text-indigo-600 focus-visible:ring-2 focus-visible:ring-indigo-500"
        >
          {t("newDesign")}
        </Link>
      </div>
      <div className="mt-4 flex flex-col justify-between gap-4 md:flex-row md:items-end">
        <div>
          <h1 className="text-2xl font-semibold tracking-[-0.035em] text-slate-950 sm:text-3xl">
            {t("resultsTitle", { symbol: transcript.symbol })}
          </h1>
          <p className="mt-2 max-w-2xl text-sm leading-6 text-slate-500">
            {t("resultsLead", { name: transcript.name })}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 text-xs text-slate-600 sm:text-sm">
          <Dna aria-hidden="true" className="size-4 text-indigo-500" />
          <span className="font-mono text-xs">{transcript.id}</span>
          <span aria-hidden="true" className="text-slate-300">/</span>
          <span>{transcript.length} nt</span>
          <span aria-hidden="true" className="text-slate-300">/</span>
          <span>CDS {cds.start}–{cds.end}</span>
        </div>
      </div>

      <div className="mt-5 grid grid-cols-3 gap-3 sm:mt-6 sm:flex sm:flex-wrap sm:gap-x-8 sm:gap-y-3">
        <SummaryMetric label={t("candidates")} value={sirnas.length.toString()} />
        <SummaryMetric label={t("passAllRules")} value={qualified.toString()} />
        <SummaryMetric
          label={t("bestScore")}
          value={
            sirnas.length === 0
              ? "—"
              : Math.max(...sirnas.map(({ score }) => score)).toFixed(2)
          }
        />
      </div>
    </header>
  );
}

function SummaryMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-col gap-0.5 sm:flex-row sm:items-baseline sm:gap-2">
      <strong className="font-mono text-lg font-semibold tabular-nums text-slate-950 sm:text-xl">
        {value}
      </strong>
      <span className="text-xs text-slate-500">{label}</span>
    </div>
  );
}
