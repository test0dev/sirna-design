"use client";

import type { KeyboardEvent } from "react";
import { ArrowUpRight } from "lucide-react";
import { useI18n } from "@/components/i18n-provider";
import { RuleBadges } from "@/components/results/rule-badges";
import type { DesignSettings, SirnaCandidate } from "@/lib/sirna-types";
import { cn } from "@/lib/utils";

interface CandidateTableProps {
  candidates: SirnaCandidate[];
  design: DesignSettings;
  selectedId: string;
  onSelect: (id: string) => void;
}

export function CandidateTable({
  candidates,
  design,
  selectedId,
  onSelect,
}: CandidateTableProps) {
  const { t } = useI18n();
  const selectWithKeyboard = (
    event: KeyboardEvent<HTMLTableRowElement>,
    id: string,
  ) => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onSelect(id);
    }
  };

  return (
    <section
      aria-labelledby="candidate-table-heading"
      className="py-6 sm:py-8"
    >
      <div className="flex flex-col justify-between gap-2 sm:flex-row sm:items-end">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-indigo-600">
            {t("rankedOutput")}
          </p>
          <h2
            id="candidate-table-heading"
            className="mt-1 text-lg font-semibold tracking-tight text-slate-950"
          >
            {t("effectiveCandidates")}
          </h2>
          <p className="mt-1 text-xs text-slate-500">
            {t("selectRow")}
          </p>
        </div>
        <p className="text-xs tabular-nums text-slate-400">
          {t("candidatesCount", { count: candidates.length })}
        </p>
      </div>

      <div className="mt-5 grid gap-2 md:hidden">
        {candidates.map((candidate) => {
          const selected = candidate.id === selectedId;
          const tmPass = candidate.seedTm <= design.seedTmMax;
          const gcPass =
            candidate.gc >= design.gcMin && candidate.gc <= design.gcMax;

          return (
            <button
              key={candidate.id}
              type="button"
              aria-pressed={selected}
              onClick={() => onSelect(candidate.id)}
              className={cn(
                "rounded-lg border border-slate-200 bg-white p-3 text-left outline-none transition-colors focus-visible:ring-2 focus-visible:ring-indigo-500",
                selected && "border-indigo-200 bg-indigo-50/70",
              )}
            >
              <span className="flex items-center justify-between gap-3">
                <span className="font-mono text-xs font-semibold text-slate-900">
                  {candidate.start}–{candidate.end}
                </span>
                <span className="flex items-center gap-3">
                  <RuleBadges candidate={candidate} compact />
                  <span className="font-mono text-xs font-semibold tabular-nums text-slate-900">
                    {candidate.score.toFixed(2)}
                  </span>
                  {selected ? (
                    <ArrowUpRight
                      aria-hidden="true"
                      className="size-4 text-indigo-600"
                    />
                  ) : null}
                </span>
              </span>
              <span className="mt-2 block break-all font-mono text-[10px] leading-4 text-slate-500">
                {candidate.sense}
              </span>
              <span className="mt-2 flex gap-4 text-[10px] text-slate-500">
                <span className={cn(!gcPass && "font-semibold text-amber-600")}>
                  GC {candidate.gc.toFixed(1)}%
                </span>
                <span className={cn(!tmPass && "font-semibold text-amber-600")}>
                  Seed Tm {candidate.seedTm.toFixed(1)}°C
                </span>
              </span>
            </button>
          );
        })}
      </div>

      <div className="mt-5 hidden overflow-x-auto rounded-lg border border-slate-200 bg-white md:block">
        <table className="w-full min-w-[1050px] border-collapse text-left">
          <thead className="bg-slate-50 text-[11px] font-medium text-slate-500">
            <tr>
              <th className="px-4 py-3">{t("position")}</th>
              <th className="px-4 py-3">{t("targetSequence")}</th>
              <th className="px-4 py-3">{t("guide53")}</th>
              <th className="px-4 py-3">{t("rules")}</th>
              <th className="px-4 py-3 text-right">GC</th>
              <th className="px-4 py-3 text-right">Seed Tm</th>
              <th className="px-4 py-3 text-right">{t("score")}</th>
              <th className="w-10 px-2 py-3">
                <span className="sr-only">{t("selected")}</span>
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-100">
            {candidates.map((candidate) => {
              const selected = candidate.id === selectedId;
              const tmPass = candidate.seedTm <= design.seedTmMax;
              const gcPass =
                candidate.gc >= design.gcMin && candidate.gc <= design.gcMax;

              return (
                <tr
                  key={candidate.id}
                  id={`candidate-${candidate.id}`}
                  tabIndex={0}
                  aria-selected={selected}
                  onClick={() => onSelect(candidate.id)}
                  onKeyDown={(event) => selectWithKeyboard(event, candidate.id)}
                  className={cn(
                    "cursor-pointer outline-none transition-colors hover:bg-slate-50 focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-indigo-500",
                    selected && "bg-indigo-50/80 hover:bg-indigo-50",
                  )}
                >
                  <td className="whitespace-nowrap px-4 py-3 font-mono text-xs font-medium text-slate-800">
                    {candidate.start}–{candidate.end}
                  </td>
                  <td className="px-4 py-3 font-mono text-[11px] tracking-tight text-slate-600">
                    {candidate.sense}
                  </td>
                  <td className="px-4 py-3 font-mono text-[11px] tracking-tight text-slate-600">
                    {candidate.antisense}
                  </td>
                  <td className="px-4 py-3">
                    <RuleBadges candidate={candidate} />
                  </td>
                  <td
                    className={cn(
                      "px-4 py-3 text-right font-mono text-xs tabular-nums",
                      gcPass ? "text-slate-600" : "font-semibold text-amber-600",
                    )}
                  >
                    {candidate.gc.toFixed(1)}%
                  </td>
                  <td
                    className={cn(
                      "px-4 py-3 text-right font-mono text-xs tabular-nums",
                      tmPass ? "text-slate-600" : "font-semibold text-amber-600",
                    )}
                  >
                    {candidate.seedTm.toFixed(1)}°C
                  </td>
                  <td className="px-4 py-3 text-right font-mono text-xs font-semibold tabular-nums text-slate-900">
                    {candidate.score.toFixed(2)}
                  </td>
                  <td className="px-2 py-3 text-indigo-600">
                    {selected ? (
                      <ArrowUpRight aria-hidden="true" className="size-4" />
                    ) : null}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </section>
  );
}
