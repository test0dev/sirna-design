"use client";

import { Check, Database } from "lucide-react";
import { useI18n } from "@/components/i18n-provider";
import type { DesignSettings } from "@/lib/sirna-types";

interface DesignParametersProps {
  design: DesignSettings;
}

export function DesignParameters({ design }: DesignParametersProps) {
  const { t } = useI18n();
  const keyParameters = [
    { label: t("combinedRuleLabel"), value: design.combine },
    { label: t("seedTmCeiling"), value: `≤ ${design.seedTmMax} °C` },
    { label: t("gcContent"), value: `${design.gcMin}–${design.gcMax}%` },
    {
      label: t("targetRange"),
      value: `${design.targetRange.from}–${design.targetRange.to}`,
    },
  ];

  return (
    <section
      aria-labelledby="design-parameters-heading"
      className="py-6 sm:py-8"
    >
      <div className="flex items-end justify-between gap-4">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-indigo-600">
            {t("inputContext")}
          </p>
          <h2
            id="design-parameters-heading"
            className="mt-1 text-lg font-semibold tracking-tight text-slate-950"
          >
            {t("designParameters")}
          </h2>
        </div>
        <span className="hidden text-xs text-slate-400 sm:block">
          {t("valuesFromJson")}
        </span>
      </div>

      <dl className="mt-5 grid grid-cols-2 border-y border-slate-200 xl:grid-cols-4">
        {keyParameters.map(({ label, value }, index) => (
          <div
            key={label}
            className={[
              "px-3 py-4 sm:px-5",
              index % 2 === 1 ? "border-l border-slate-200" : "",
              index > 1 ? "border-t border-slate-200 xl:border-t-0" : "",
              index > 0 ? "xl:border-l xl:border-slate-200" : "",
              index === 0 ? "pl-0 sm:pl-0" : "",
            ].join(" ")}
          >
            <dt className="text-xs text-slate-500">{label}</dt>
            <dd className="mt-1.5 text-sm font-medium text-slate-900">{value}</dd>
          </div>
        ))}
      </dl>

      <div className="mt-5 grid gap-x-12 gap-y-4 text-sm md:grid-cols-2">
        <ParameterRow
          label={t("specificityCheck")}
          value={
            <span className="inline-flex items-center gap-2">
              <Database aria-hidden="true" className="size-3.5 text-slate-400" />
              {design.specificity.species}, {design.specificity.database}
            </span>
          }
        />
        <ParameterRow
          label={t("duplex")}
          value={t("duplexValue", {
            length: design.length,
            overhang: design.overhang,
          })}
        />
        <ParameterRow
          label={t("avoidContiguous")}
          value={`G/C ≥ ${design.avoidContiguousGC} nt · A/T ≥ ${design.avoidContiguousAT} nt`}
        />
        <ParameterRow
          label={t("specificityFilters")}
          value={`${design.specificity.hideLessSpecific ? t("hideLessSpecificShort") : t("showAll")} · ${
            design.specificity.showOffTargetHits
              ? t("showHits")
              : t("hideHits")
          }`}
        />
      </div>

      <div className="mt-5 flex flex-wrap items-center gap-2">
        <span className="mr-1 text-xs text-slate-500">{t("enabledAlgorithms")}</span>
        {design.algorithms.map((algorithm) => (
          <span
            key={algorithm}
            className="inline-flex h-7 items-center gap-1.5 rounded-md bg-slate-100 px-2.5 text-xs font-medium text-slate-700"
          >
            <Check aria-hidden="true" className="size-3 text-emerald-600" />
            {algorithm}
          </span>
        ))}
      </div>
    </section>
  );
}

function ParameterRow({
  label,
  value,
}: {
  label: string;
  value: React.ReactNode;
}) {
  return (
    <div className="grid gap-1 sm:grid-cols-[9.5rem_1fr] sm:gap-4">
      <span className="text-xs text-slate-500">{label}</span>
      <span className="text-xs leading-5 text-slate-700">{value}</span>
    </div>
  );
}
