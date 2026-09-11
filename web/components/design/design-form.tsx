"use client";

import { FlaskConical, LoaderCircle } from "lucide-react";
import { useMemo, useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { HelpTip } from "@/components/design/help-tip";
import { SectionHeading } from "@/components/design/section-heading";
import { useI18n } from "@/components/i18n-provider";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Textarea } from "@/components/ui/textarea";
import {
  ALGORITHM_META,
  COMBINE_RULES,
  DESIGN_INPUT_STORAGE_KEY,
  DESIGN_RESULT_STORAGE_KEY,
  MAX_SEQUENCE_LENGTH,
  SAMPLE_ACCESSION,
  SAMPLE_GENE_SYMBOL,
  SAMPLE_SEQUENCE,
  SPECIFICITY_OPTIONS,
  defaultDesignInput,
  parseNucleotideSequence,
  validateDesignInput,
  type CombineRule,
  type ContiguousLength,
  type DesignInput,
} from "@/lib/design-input";
import {
  combineHintKey,
  localizeThrownMessage,
  specificityOptionLabel,
} from "@/lib/i18n";
import type { SirnaResult } from "@/lib/sirna-types";
import { cn } from "@/lib/utils";

const CONTIGUOUS_LENGTHS: ContiguousLength[] = [4, 5, 6, 7];

export function DesignForm() {
  const router = useRouter();
  const { t } = useI18n();
  const [input, setInput] = useState<DesignInput>(defaultDesignInput);
  const [error, setError] = useState<string | null>(null);
  const [retrieveMessage, setRetrieveMessage] = useState<string | null>(null);
  const [resolveSummary, setResolveSummary] = useState<string | null>(null);
  const [retrieving, setRetrieving] = useState(false);
  const [resolving, setResolving] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  const parsed = useMemo(
    () => parseNucleotideSequence(input.sequence),
    [input.sequence],
  );

  const update = <K extends keyof DesignInput>(key: K, value: DesignInput[K]) => {
    setInput((current) => ({ ...current, [key]: value }));
    setError(null);
  };

  const resolveTarget = async () => {
    const symbol = input.geneSymbol.trim();
    if (!symbol) {
      setRetrieveMessage(t("errGeneEmpty"));
      return;
    }

    setResolving(true);
    setRetrieveMessage(null);
    setResolveSummary(null);
    setError(null);

    try {
      const response = await fetch(
        `/api/resolve-target?symbol=${encodeURIComponent(symbol)}&fetchSequence=1`,
      );
      const payload = (await response.json()) as {
        gene?: { symbol?: string };
        mane_select_transcript?: {
          ensembl_transcript?: string;
          refseq?: string | null;
          mane_select?: boolean;
          canonical?: boolean;
        };
        accession?: string;
        sequence?: string;
        header?: string;
        length?: number;
        error?: string;
      };

      if (!response.ok || !payload.sequence) {
        throw new Error(payload.error ?? t("errResolveFailed"));
      }

      const refseq =
        payload.accession ??
        payload.mane_select_transcript?.refseq ??
        input.accession;
      setInput((current) => ({
        ...current,
        geneSymbol: payload.gene?.symbol ?? symbol,
        accession: refseq,
        sequence: payload.sequence!,
      }));
      setResolveSummary(
        t("resolveSummary", {
          symbol: payload.gene?.symbol ?? symbol,
          transcript:
            payload.mane_select_transcript?.ensembl_transcript ?? "—",
          refseq: refseq || "—",
          tag: payload.mane_select_transcript?.mane_select
            ? t("maneSelectTag")
            : t("canonicalTag"),
        }),
      );
      setRetrieveMessage(
        t("loadedTranscript", {
          header: payload.header ?? refseq,
          length: Number(payload.length ?? 0).toLocaleString(),
        }),
      );
    } catch (resolveError) {
      setRetrieveMessage(
        resolveError instanceof Error
          ? localizeThrownMessage(resolveError.message, t)
          : t("errResolveFailed"),
      );
    } finally {
      setResolving(false);
    }
  };

  const retrieveSequence = async () => {
    const accession = input.accession.trim();
    if (!accession) {
      setRetrieveMessage(t("errAccessionEmpty"));
      return;
    }

    setRetrieving(true);
    setRetrieveMessage(null);
    setError(null);

    try {
      const response = await fetch(
        `/api/retrieve?accession=${encodeURIComponent(accession)}`,
      );
      const payload = (await response.json()) as {
        sequence?: string;
        header?: string;
        length?: number;
        accession?: string;
        error?: string;
      };

      if (!response.ok || !payload.sequence) {
        throw new Error(payload.error ?? t("errRetrieveFailed"));
      }

      setInput((current) => ({
        ...current,
        accession: payload.accession ?? accession,
        sequence: payload.sequence!,
      }));
      setRetrieveMessage(
        t("loadedTranscript", {
          header: payload.header ?? accession,
          length: Number(payload.length ?? 0).toLocaleString(),
        }),
      );
    } catch (retrieveError) {
      setRetrieveMessage(
        retrieveError instanceof Error
          ? localizeThrownMessage(retrieveError.message, t)
          : t("errRetrieveFailed"),
      );
    } finally {
      setRetrieving(false);
    }
  };

  const loadSample = () => {
    setInput({
      ...defaultDesignInput,
      geneSymbol: SAMPLE_GENE_SYMBOL,
      accession: SAMPLE_ACCESSION,
      sequence: SAMPLE_SEQUENCE,
    });
    setResolveSummary(null);
    setRetrieveMessage(t("loadedSample"));
    setError(null);
  };

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const nextError = validateDesignInput(input);
    if (nextError) {
      setError(nextError);
      return;
    }

    setSubmitting(true);
    setError(null);
    window.sessionStorage.setItem(DESIGN_INPUT_STORAGE_KEY, JSON.stringify(input));

    try {
      const response = await fetch("/api/design", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
      });
      const payload = (await response.json()) as {
        result?: SirnaResult;
        error?: string;
      };

      if (!response.ok || !payload.result) {
        throw new Error(payload.error ?? t("errDesignFailed"));
      }

      window.sessionStorage.setItem(
        DESIGN_RESULT_STORAGE_KEY,
        JSON.stringify(payload.result),
      );
      router.push("/results");
    } catch (submitError) {
      setSubmitting(false);
      setError(
        submitError instanceof Error
          ? localizeThrownMessage(submitError.message, t)
          : t("errDesignFailed"),
      );
    }
  };

  return (
    <form onSubmit={submit}>
      <header className="border-b border-slate-200 pb-6 sm:pb-7">
        <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.18em] text-indigo-600">
          <FlaskConical aria-hidden="true" className="size-4" />
          {t("brand")}
        </div>
        <div className="mt-4 flex flex-col justify-between gap-4 md:flex-row md:items-end">
          <div>
            <h1 className="text-2xl font-semibold tracking-[-0.035em] text-slate-950 sm:text-3xl">
              {t("designTitle")}
            </h1>
            <p className="mt-2 max-w-2xl text-sm leading-6 text-slate-500">
              {t("designLead")}
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 text-xs text-slate-600 sm:text-sm">
            <span>{t("fastaOrPlain")}</span>
            <span aria-hidden="true" className="text-slate-300">
              /
            </span>
            <span>{t("upToNt", { max: MAX_SEQUENCE_LENGTH.toLocaleString() })}</span>
          </div>
        </div>
      </header>

      <section aria-labelledby="sequence-heading" className="py-6 sm:py-8">
        <SectionHeading
          id="sequence-heading"
          eyebrow={t("targetInput")}
          title={t("sequence")}
          description={t("sequenceHelp")}
          action={
            <Button type="button" variant="ghost" size="sm" onClick={loadSample}>
              {t("loadSample")}
            </Button>
          }
        />

        <div className="mt-5 grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
          <div className="grid gap-2">
            <Label htmlFor="gene-symbol" className="text-xs text-slate-500">
              {t("geneSymbol")}
            </Label>
            <Input
              id="gene-symbol"
              name="geneSymbol"
              value={input.geneSymbol}
              onChange={(event) => update("geneSymbol", event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  void resolveTarget();
                }
              }}
              placeholder="PCSK9"
              autoComplete="off"
              spellCheck={false}
            />
          </div>
          <Button
            type="button"
            variant="default"
            onClick={() => void resolveTarget()}
            disabled={resolving || retrieving}
          >
            {resolving ? (
              <LoaderCircle className="size-4 animate-spin" />
            ) : null}
            {t("resolveAndLoad")}
          </Button>
        </div>

        <div className="mt-4 grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
          <div className="grid gap-2">
            <Label htmlFor="accession" className="text-xs text-slate-500">
              {t("accessionNumber")}
            </Label>
            <Input
              id="accession"
              name="accession"
              value={input.accession}
              onChange={(event) => update("accession", event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  void retrieveSequence();
                }
              }}
              placeholder="NM_012131"
              autoComplete="off"
              spellCheck={false}
            />
          </div>
          <Button
            type="button"
            variant="outline"
            onClick={() => void retrieveSequence()}
            disabled={retrieving || resolving}
          >
            {retrieving ? (
              <LoaderCircle className="size-4 animate-spin" />
            ) : null}
            {t("retrieveSequence")}
          </Button>
        </div>

        {resolveSummary ? (
          <p className="mt-2 font-mono text-xs text-slate-600">{resolveSummary}</p>
        ) : null}
        {retrieveMessage ? (
          <p
            className={
              retrieveMessage.startsWith("Loaded ") ||
              retrieveMessage.startsWith("已载入")
                ? "mt-2 text-xs text-slate-500"
                : "mt-2 text-xs text-rose-600"
            }
          >
            {retrieveMessage}
          </p>
        ) : null}

        <div className="mt-5 grid gap-2">
          <Label htmlFor="sequence" className="text-xs text-slate-500">
            {t("sequence")}
          </Label>
          <Textarea
            id="sequence"
            name="sequence"
            value={input.sequence}
            onChange={(event) => update("sequence", event.target.value)}
            spellCheck={false}
            className="min-h-52 font-mono text-[13px] leading-6"
            placeholder={">sequence_name\nATGC..."}
          />
        </div>

        <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-slate-500">
          <strong className="font-mono text-sm font-semibold tabular-nums text-slate-950">
            {parsed.length.toLocaleString()}
          </strong>
          <span>nt</span>
          {parsed.gcPercent !== null ? (
            <>
              <span aria-hidden="true" className="text-slate-300">
                /
              </span>
              <span>GC {parsed.gcPercent.toFixed(1)}%</span>
            </>
          ) : null}
          {parsed.header ? (
            <>
              <span aria-hidden="true" className="text-slate-300">
                /
              </span>
              <span className="font-mono">{parsed.header}</span>
            </>
          ) : null}
          {parsed.invalidChars.length > 0 ? (
            <span className="text-rose-600">
              {t("invalidChars", { chars: parsed.invalidChars.join(" ") })}
            </span>
          ) : null}
          {parsed.length > MAX_SEQUENCE_LENGTH ? (
            <span className="text-rose-600">{t("exceedsLimit")}</span>
          ) : null}
        </div>
      </section>

      <section
        aria-labelledby="algorithm-heading"
        className="border-t border-slate-200 py-6 sm:py-8"
      >
        <SectionHeading
          id="algorithm-heading"
          eyebrow={t("functionalSelection")}
          title={t("efficacyAlgorithms")}
          description={t("efficacyHelp")}
        />

        <div className="mt-5 grid gap-2 md:grid-cols-3">
          {ALGORITHM_META.map((algorithm) => {
            const selected = input.algorithms[algorithm.key];
            return (
              <button
                key={algorithm.key}
                type="button"
                role="checkbox"
                aria-checked={selected}
                onClick={() =>
                  update("algorithms", {
                    ...input.algorithms,
                    [algorithm.key]: !selected,
                  })
                }
                className={cn(
                  "rounded-lg border p-3 text-left outline-none transition-colors focus-visible:ring-2 focus-visible:ring-indigo-500",
                  selected
                    ? "border-indigo-200 bg-indigo-50/70"
                    : "border-slate-200 bg-white",
                )}
              >
                <span className="flex items-center justify-between gap-3">
                  <span className="flex items-center gap-2">
                    <span
                      className={cn(
                        "inline-flex size-5 items-center justify-center rounded-[4px] text-[10px] font-semibold",
                        algorithm.className,
                      )}
                    >
                      {algorithm.short}
                    </span>
                    <span className="text-sm font-medium text-slate-900">
                      {algorithm.key}
                    </span>
                  </span>
                  <span
                    className={cn(
                      "text-[11px]",
                      selected ? "text-indigo-600" : "text-slate-400",
                    )}
                  >
                    {selected ? t("on") : t("off")}
                  </span>
                </span>
                <a
                  href={algorithm.href}
                  target="_blank"
                  rel="noreferrer"
                  onClick={(event) => event.stopPropagation()}
                  className="mt-2 inline-block text-xs text-slate-500 underline-offset-2 hover:text-indigo-600 hover:underline"
                >
                  {algorithm.citation}
                </a>
              </button>
            );
          })}
        </div>

        <div className="mt-6">
          <div className="flex items-center gap-1.5">
            <p className="text-xs text-slate-500">{t("combinedRule")}</p>
            <HelpTip label={t("combinedRule")}>
              <p>{t("combinedRuleHelp")}</p>
            </HelpTip>
          </div>
          <RadioGroup
            value={input.combine}
            onValueChange={(value) => update("combine", value as CombineRule)}
            className="mt-3 gap-0"
          >
            {COMBINE_RULES.map((rule, index) => (
              <Label
                key={rule.value}
                htmlFor={`combine-${index}`}
                className="flex cursor-pointer items-start gap-3 border-t border-slate-200 py-3 font-normal last:border-b"
              >
                <RadioGroupItem
                  id={`combine-${index}`}
                  value={rule.value}
                  className="mt-0.5"
                />
                <span>
                  <span className="block font-mono text-sm text-slate-900">
                    {rule.value}
                  </span>
                  <span className="mt-1 block text-xs text-slate-500">
                    {t(combineHintKey(rule.value))}
                  </span>
                </span>
              </Label>
            ))}
          </RadioGroup>
        </div>
      </section>

      <section
        aria-labelledby="seed-tm-heading"
        className="border-t border-slate-200 py-6 sm:py-8"
      >
        <SectionHeading
          id="seed-tm-heading"
          eyebrow={t("offTarget")}
          title={t("seedDuplex")}
          description={t("seedDuplexHelp")}
        />

        <dl className="mt-5 grid grid-cols-2 border-y border-slate-200 xl:grid-cols-4">
          <div className="px-0 py-4 sm:pr-5">
            <dt className="flex items-center gap-1.5 text-xs text-slate-500">
              {t("maxTm")}
              <HelpTip label={t("maxTm")}>
                <p>{t("seedTmHelp")}</p>
                <a
                  href="https://doi.org/10.1093/nar/gkn902"
                  target="_blank"
                  rel="noreferrer"
                  className="mt-2 inline-block text-indigo-600 hover:underline"
                >
                  Ui-Tei et al., NAR 2008
                </a>
              </HelpTip>
            </dt>
            <dd className="mt-2 flex items-center gap-2">
              <Input
                id="seed-tm"
                type="number"
                step="0.1"
                inputMode="decimal"
                value={input.seedTmMax}
                onChange={(event) =>
                  update("seedTmMax", Number(event.target.value))
                }
                className="w-24"
              />
              <span className="text-sm text-slate-500">°C</span>
            </dd>
          </div>
          <div className="border-l border-slate-200 px-3 py-4 sm:px-5">
            <dt className="text-xs text-slate-500">{t("recommended")}</dt>
            <dd className="mt-1.5 text-sm font-medium text-slate-900">
              ≤ 21.5 °C
            </dd>
          </div>
          <div className="border-t border-slate-200 px-0 py-4 sm:pr-5 xl:border-t-0 xl:border-l xl:px-5">
            <dt className="text-xs text-slate-500">{t("seedRegion")}</dt>
            <dd className="mt-1.5 text-sm font-medium text-slate-900">
              {t("guide28")}
            </dd>
          </div>
          <div className="border-t border-l border-slate-200 px-3 py-4 sm:px-5 xl:border-t-0">
            <dt className="text-xs text-slate-500">{t("checkedStrands")}</dt>
            <dd className="mt-1.5 text-sm font-medium text-slate-900">
              {t("guideAndPassenger")}
            </dd>
          </div>
        </dl>
      </section>

      <section
        aria-labelledby="specificity-heading"
        className="border-t border-slate-200 py-6 sm:py-8"
      >
        <SectionHeading
          id="specificity-heading"
          eyebrow={t("specificity")}
          title={t("transcriptDb")}
          description={t("transcriptDbHelp")}
        />

        <div className="mt-5 grid gap-2">
          <Label htmlFor="specificity" className="text-xs text-slate-500">
            {t("specificityCheck")}
          </Label>
          <select
            id="specificity"
            value={input.specificity}
            onChange={(event) => update("specificity", event.target.value)}
            className="h-9 w-full rounded-md border border-slate-200 bg-white px-3 text-sm text-slate-900 outline-none focus-visible:border-indigo-300 focus-visible:ring-2 focus-visible:ring-indigo-500"
          >
            {SPECIFICITY_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {specificityOptionLabel(option.value, option.label, t)}
              </option>
            ))}
          </select>
        </div>

        <div className="mt-5 grid gap-3 md:grid-cols-2">
          <CheckboxRow
            id="hide-less-specific"
            checked={input.hideLessSpecific}
            onCheckedChange={(checked) => update("hideLessSpecific", checked)}
            label={t("hideLessSpecific")}
            hint={t("hideLessSpecificHint")}
          />
          <CheckboxRow
            id="show-off-target-hits"
            checked={input.showOffTargetHits}
            onCheckedChange={(checked) => update("showOffTargetHits", checked)}
            label={t("showOffTargetHits")}
            hint={t("showOffTargetHitsHint")}
          />
        </div>
      </section>

      <section
        aria-labelledby="other-options-heading"
        className="border-t border-slate-200 py-6 sm:py-8"
      >
        <SectionHeading
          id="other-options-heading"
          eyebrow={t("constraints")}
          title={t("otherOptions")}
          description={t("otherOptionsHelp")}
        />

        <div className="mt-5 grid gap-x-12 gap-y-5 md:grid-cols-2">
          <RangeField
            label={t("targetRange")}
            fromId="target-from"
            toId="target-to"
            fromValue={input.targetRangeFrom}
            toValue={input.targetRangeTo}
            fromPlaceholder="1"
            toPlaceholder={parsed.length ? String(parsed.length) : "end"}
            onFromChange={(value) => update("targetRangeFrom", value)}
            onToChange={(value) => update("targetRangeTo", value)}
            suffix="nt"
          />
          <RangeField
            label={t("gcContent")}
            fromId="gc-min"
            toId="gc-max"
            fromValue={input.gcMin}
            toValue={input.gcMax}
            fromPlaceholder="30"
            toPlaceholder="52"
            onFromChange={(value) => update("gcMin", value)}
            onToChange={(value) => update("gcMax", value)}
            suffix="%"
          />
        </div>

        <div className="mt-5 grid gap-4 md:grid-cols-2">
          <ContiguousControl
            id="avoid-gc"
            enabled={input.avoidContiguousGC}
            length={input.avoidContiguousGCMin}
            onEnabledChange={(checked) => update("avoidContiguousGC", checked)}
            onLengthChange={(value) => update("avoidContiguousGCMin", value)}
            label={t("avoidGC")}
            hint={t("avoidGCHint")}
            unitLabel={t("ntOrMore")}
          />
          <ContiguousControl
            id="avoid-at"
            enabled={input.avoidContiguousAT}
            length={input.avoidContiguousATMin}
            onEnabledChange={(checked) => update("avoidContiguousAT", checked)}
            onLengthChange={(value) => update("avoidContiguousATMin", value)}
            label={t("avoidAT")}
            hint={t("avoidATHint")}
            unitLabel={t("ntOrMore")}
          />
        </div>

        <div className="mt-5 grid gap-x-12 gap-y-5 md:grid-cols-2">
          <PatternField
            enabled={input.customPatternEnabled}
            onEnabledChange={(checked) =>
              update("customPatternEnabled", checked)
            }
            id="custom-pattern"
            label={t("customPattern")}
            value={input.customPattern}
            onChange={(value) => update("customPattern", value)}
            placeholder="NNGNNNNNNNNNNNNNNNNNNNN"
            help={<p>{t("customPatternHelp")}</p>}
          />
          <PatternField
            enabled={input.excludePatternEnabled}
            onEnabledChange={(checked) =>
              update("excludePatternEnabled", checked)
            }
            id="exclude-pattern"
            label={t("excludePattern")}
            value={input.excludePattern}
            onChange={(value) => update("excludePattern", value)}
            placeholder="TTTT"
            help={<p>{t("excludePatternHelp")}</p>}
          />
        </div>
      </section>

      <div className="flex flex-col gap-4 border-t border-slate-200 pt-6 sm:flex-row sm:items-center sm:justify-between sm:pt-7">
        <CheckboxRow
          id="match-all"
          checked={input.matchAllCriteria}
          onCheckedChange={(checked) => update("matchAllCriteria", checked)}
          label={t("matchAll")}
          hint={t("matchAllHint")}
        />
        <div className="flex shrink-0 flex-col items-stretch gap-2 sm:items-end">
          {error ? (
            <p role="alert" className="text-xs text-rose-600 sm:text-right">
              {error}
            </p>
          ) : (
            <p className="text-xs text-slate-400 sm:text-right">
              {t("submitHint")}
            </p>
          )}
          <Button type="submit" size="lg" disabled={submitting}>
            {submitting ? <LoaderCircle className="size-4 animate-spin" /> : null}
            {t("designSirna")}
          </Button>
        </div>
      </div>
    </form>
  );
}

function CheckboxRow({
  id,
  checked,
  onCheckedChange,
  label,
  hint,
}: {
  id: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  label: string;
  hint: string;
}) {
  return (
    <div className="flex items-start gap-3">
      <Checkbox
        id={id}
        checked={checked}
        onCheckedChange={(value) => onCheckedChange(value === true)}
        className="mt-0.5"
      />
      <Label htmlFor={id} className="grid gap-1 font-normal">
        <span className="text-sm text-slate-800">{label}</span>
        <span className="text-xs leading-5 font-normal text-slate-500">
          {hint}
        </span>
      </Label>
    </div>
  );
}

function RangeField({
  label,
  fromId,
  toId,
  fromValue,
  toValue,
  fromPlaceholder,
  toPlaceholder,
  onFromChange,
  onToChange,
  suffix,
}: {
  label: string;
  fromId: string;
  toId: string;
  fromValue: string;
  toValue: string;
  fromPlaceholder: string;
  toPlaceholder: string;
  onFromChange: (value: string) => void;
  onToChange: (value: string) => void;
  suffix: string;
}) {
  return (
    <div>
      <p className="text-xs text-slate-500">{label}</p>
      <div className="mt-2 flex items-center gap-2">
        <Input
          id={fromId}
          inputMode="numeric"
          value={fromValue}
          placeholder={fromPlaceholder}
          onChange={(event) => onFromChange(event.target.value)}
          className="w-24"
        />
        <span className="text-xs text-slate-400">–</span>
        <Input
          id={toId}
          inputMode="numeric"
          value={toValue}
          placeholder={toPlaceholder}
          onChange={(event) => onToChange(event.target.value)}
          className="w-24"
        />
        <span className="text-xs text-slate-500">{suffix}</span>
      </div>
    </div>
  );
}

function ContiguousControl({
  id,
  enabled,
  length,
  onEnabledChange,
  onLengthChange,
  label,
  hint,
  unitLabel,
}: {
  id: string;
  enabled: boolean;
  length: ContiguousLength;
  onEnabledChange: (checked: boolean) => void;
  onLengthChange: (value: ContiguousLength) => void;
  label: string;
  hint: string;
  unitLabel: string;
}) {
  const groupId = id;

  return (
    <div
      className={cn(
        "rounded-lg border p-3",
        enabled ? "border-slate-200" : "border-slate-200 bg-slate-50/70",
      )}
    >
      <div className="flex items-start gap-3">
        <Checkbox
          id={groupId}
          checked={enabled}
          onCheckedChange={(value) => onEnabledChange(value === true)}
          className="mt-0.5"
        />
        <Label htmlFor={groupId} className="grid gap-1 font-normal">
          <span className="text-sm text-slate-800">{label}</span>
          <span className="text-xs font-normal text-slate-500">{hint}</span>
        </Label>
      </div>
      <div className="mt-3 flex flex-wrap items-center gap-2 pl-7">
        {CONTIGUOUS_LENGTHS.map((value) => {
          const selected = enabled && length === value;
          return (
            <button
              key={value}
              type="button"
              disabled={!enabled}
              aria-pressed={selected}
              onClick={() => onLengthChange(value)}
              className={cn(
                "inline-flex h-8 min-w-8 items-center justify-center rounded-md border px-2.5 font-mono text-xs outline-none transition-colors focus-visible:ring-2 focus-visible:ring-indigo-500 disabled:opacity-40",
                selected
                  ? "border-indigo-200 bg-indigo-50 text-indigo-700"
                  : "border-slate-200 bg-white text-slate-600 hover:bg-slate-50",
              )}
            >
              {value}
            </button>
          );
        })}
        <span className="text-xs text-slate-500">{unitLabel}</span>
      </div>
    </div>
  );
}

function PatternField({
  enabled,
  onEnabledChange,
  id,
  label,
  value,
  onChange,
  placeholder,
  help,
}: {
  enabled: boolean;
  onEnabledChange: (checked: boolean) => void;
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  help: React.ReactNode;
}) {
  return (
    <div>
      <div className="flex items-center gap-2">
        <Checkbox
          id={`${id}-enabled`}
          checked={enabled}
          onCheckedChange={(next) => onEnabledChange(next === true)}
        />
        <Label htmlFor={`${id}-enabled`} className="text-xs text-slate-500">
          {label}
        </Label>
        <HelpTip label={`${label} help`}>{help}</HelpTip>
      </div>
      <Input
        id={id}
        value={value}
        disabled={!enabled}
        placeholder={placeholder}
        spellCheck={false}
        onChange={(event) => onChange(event.target.value)}
        className="mt-2 font-mono"
      />
    </div>
  );
}
