"use client";

import { Check, CircleAlert, X } from "lucide-react";
import { useI18n } from "@/components/i18n-provider";
import { RuleBadges } from "@/components/results/rule-badges";
import type {
  DesignRuleName,
  DesignSettings,
  SirnaCandidate,
} from "@/lib/sirna-types";
import { cn } from "@/lib/utils";

const ruleNames: DesignRuleName[] = [
  "Ui-Tei",
  "Reynolds",
  "Amarzguioui",
];

export function CandidateDetailTip({
  candidate,
  design,
}: {
  candidate: SirnaCandidate;
  design: DesignSettings;
}) {
  const { t } = useI18n();
  const gcPass =
    candidate.gc >= design.gcMin && candidate.gc <= design.gcMax;
  const tmPass = candidate.seedTm <= design.seedTmMax;
  const qualified =
    gcPass && tmPass && Object.values(candidate.rules).every(Boolean);

  return (
    <div>
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="font-mono text-[10px] font-semibold uppercase tracking-wider text-indigo-600">
            {candidate.id}
          </p>
          <h3 className="mt-1 text-base font-semibold tracking-tight text-slate-950">
            siRNA {candidate.start}–{candidate.end}
          </h3>
        </div>
        <span
          className={cn(
            "inline-flex h-6 shrink-0 items-center gap-1 rounded-md px-2 text-[10px] font-medium",
            qualified
              ? "bg-emerald-50 text-emerald-700"
              : "bg-amber-50 text-amber-700",
          )}
        >
          {qualified ? (
            <Check aria-hidden="true" className="size-3" />
          ) : (
            <CircleAlert aria-hidden="true" className="size-3" />
          )}
          {qualified ? t("passed") : t("review")}
        </span>
      </div>

      <div className="mt-4 grid grid-cols-3 divide-x divide-slate-200 border-y border-slate-200 py-3">
        <Metric label={t("score")} value={candidate.score.toFixed(2)} />
        <Metric
          label="GC"
          value={`${candidate.gc.toFixed(1)}%`}
          passed={gcPass}
        />
        <Metric
          label="Seed Tm"
          value={`${candidate.seedTm.toFixed(1)}°C`}
          passed={tmPass}
        />
      </div>

      <div className="mt-4 space-y-3">
        <Sequence label={t("targetSense")} sequence={candidate.sense} />
        <Sequence label={t("guideAntisense")} sequence={candidate.antisense} />
      </div>

      <div className="mt-4 flex items-center justify-between">
        <span className="text-[11px] font-medium text-slate-500">
          {t("designRules")}
        </span>
        <RuleBadges candidate={candidate} />
      </div>
      <div className="mt-2 grid grid-cols-3 gap-1.5">
        {ruleNames.map((rule) => (
          <RuleStatus
            key={rule}
            label={rule}
            passed={candidate.rules[rule]}
          />
        ))}
      </div>
    </div>
  );
}

function Sequence({ label, sequence }: { label: string; sequence: string }) {
  return (
    <div>
      <p className="mb-1 text-[10px] text-slate-400">{label}</p>
      <p className="overflow-x-auto rounded-md bg-slate-950 px-3 py-2 font-mono text-[10px] tracking-[0.04em] text-slate-100">
        {sequence}
      </p>
    </div>
  );
}

function Metric({
  label,
  value,
  passed,
}: {
  label: string;
  value: string;
  passed?: boolean;
}) {
  return (
    <div className="px-2 text-center">
      <p className="text-[10px] text-slate-400">{label}</p>
      <p
        className={cn(
          "mt-1 font-mono text-sm font-semibold tabular-nums text-slate-900",
          passed === false && "text-amber-600",
        )}
      >
        {value}
      </p>
    </div>
  );
}

function RuleStatus({
  label,
  passed,
}: {
  label: string;
  passed: boolean;
}) {
  return (
    <span
      className={cn(
        "inline-flex min-w-0 items-center justify-center gap-1 rounded-md bg-slate-50 px-1.5 py-2 text-[9px] font-medium",
        passed ? "text-emerald-700" : "text-slate-400",
      )}
    >
      {passed ? (
        <Check aria-hidden="true" className="size-3 shrink-0" />
      ) : (
        <X aria-hidden="true" className="size-3 shrink-0" />
      )}
      <span className="truncate">{label}</span>
    </span>
  );
}
