"use client";

import { Fragment, useMemo, useState } from "react";
import { ChevronDown, ChevronUp } from "lucide-react";
import { useI18n } from "@/components/i18n-provider";
import { CandidateDetailTip } from "@/components/results/candidate-detail-tip";
import { RuleBadges } from "@/components/results/rule-badges";
import {
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from "@/components/ui/hover-card";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  getTmBand,
  type DesignSettings,
  type SirnaCandidate,
  type SirnaResult,
  type TmBand,
} from "@/lib/sirna-types";
import { cn } from "@/lib/utils";

/** 连续无 siRNA 行数超过该阈值时，默认折叠。 */
const EMPTY_GAP_COLLAPSE_AFTER = 2;

const tmBandClasses: Record<TmBand, string> = {
  "under-10": "bg-slate-800 text-white",
  "under-15": "bg-slate-600 text-white",
  "under-21.5": "bg-indigo-500 text-white",
  functional: "border border-slate-300 bg-white text-slate-700",
};

interface TranscriptMapProps {
  result: SirnaResult;
  selectedId: string;
  onSelect: (id: string) => void;
}

interface CandidateSegment {
  candidate: SirnaCandidate;
  segmentStart: number;
  segmentEnd: number;
  lane: number;
  showDetails: boolean;
  continuesFromPreviousRow: boolean;
  continuesToNextRow: boolean;
}

interface TranscriptRowData {
  start: number;
  end: number;
  sequence: string;
  segments: CandidateSegment[];
}

type RowBlock =
  | { type: "rows"; rows: TranscriptRowData[] }
  | { type: "gap"; key: string; rows: TranscriptRowData[] };

export function TranscriptMap({
  result,
  selectedId,
  onSelect,
}: TranscriptMapProps) {
  const { t } = useI18n();

  return (
    <section
      aria-labelledby="transcript-map-heading"
      className="py-6 sm:py-8"
    >
      <div>
        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-indigo-600">
          {t("sequenceContext")}
        </p>
        <h2
          id="transcript-map-heading"
          className="mt-1 text-lg font-semibold tracking-tight text-slate-950"
        >
          {t("graphicalView")}
        </h2>
        <p className="mt-1 text-xs text-slate-500">
          <span className="sm:hidden">{t("wrapHelp", { n: 50 })}</span>
          <span className="hidden sm:inline">{t("wrapHelp", { n: 100 })}</span>
        </p>
      </div>

      <TmLegend />

      <div className="mt-5 sm:hidden">
        <TranscriptRows
          result={result}
          basesPerRow={50}
          selectedId={selectedId}
          onSelect={onSelect}
          compact
        />
      </div>

      <div className="mt-5 hidden overflow-x-auto sm:block">
        <div className="min-w-[1080px]">
          <TranscriptRows
            result={result}
            basesPerRow={100}
            selectedId={selectedId}
            onSelect={onSelect}
          />
        </div>
      </div>
    </section>
  );
}

function TranscriptRows({
  result,
  basesPerRow,
  selectedId,
  onSelect,
  compact = false,
}: TranscriptMapProps & { basesPerRow: number; compact?: boolean }) {
  const { transcript, sirnas, cds } = result;
  const [expandedGaps, setExpandedGaps] = useState<Set<string>>(() => new Set());

  const rows = useMemo(
    () =>
      Array.from(
        { length: Math.ceil(transcript.length / basesPerRow) },
        (_, rowIndex) => {
          const start = rowIndex * basesPerRow + 1;
          const end = Math.min(start + basesPerRow - 1, transcript.length);
          return {
            start,
            end,
            sequence: transcript.sequence.slice(start - 1, end),
            segments: getSegmentsForRow(sirnas, start, end, basesPerRow),
          };
        },
      ),
    [transcript.length, transcript.sequence, sirnas, basesPerRow],
  );

  const blocks = useMemo(() => groupRowBlocks(rows), [rows]);

  const toggleGap = (key: string) => {
    setExpandedGaps((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  return blocks.map((block) => {
    if (block.type === "rows") {
      return (
        <Fragment key={`rows-${block.rows[0].start}`}>
          {block.rows.map((row) => (
            <TranscriptRow
              key={row.start}
              row={row}
              design={result.design}
              cdsStart={cds.start}
              cdsEnd={cds.end}
              selectedId={selectedId}
              onSelect={onSelect}
              basesPerRow={basesPerRow}
              compact={compact}
            />
          ))}
        </Fragment>
      );
    }

    const expanded = expandedGaps.has(block.key);
    return (
      <EmptyGapBlock
        key={block.key}
        rows={block.rows}
        expanded={expanded}
        onToggle={() => toggleGap(block.key)}
        design={result.design}
        cdsStart={cds.start}
        cdsEnd={cds.end}
        selectedId={selectedId}
        onSelect={onSelect}
        basesPerRow={basesPerRow}
        compact={compact}
      />
    );
  });
}

function groupRowBlocks(rows: TranscriptRowData[]): RowBlock[] {
  const blocks: RowBlock[] = [];
  let emptyBuffer: TranscriptRowData[] = [];
  let coveredBuffer: TranscriptRowData[] = [];

  const flushCovered = () => {
    if (coveredBuffer.length === 0) return;
    blocks.push({ type: "rows", rows: coveredBuffer });
    coveredBuffer = [];
  };

  const flushEmpty = () => {
    if (emptyBuffer.length === 0) return;
    if (emptyBuffer.length > EMPTY_GAP_COLLAPSE_AFTER) {
      blocks.push({
        type: "gap",
        key: `${emptyBuffer[0].start}-${emptyBuffer[emptyBuffer.length - 1].end}`,
        rows: emptyBuffer,
      });
    } else {
      coveredBuffer.push(...emptyBuffer);
    }
    emptyBuffer = [];
  };

  for (const row of rows) {
    if (row.segments.length === 0) {
      flushCovered();
      emptyBuffer.push(row);
    } else {
      flushEmpty();
      coveredBuffer.push(row);
    }
  }
  flushEmpty();
  flushCovered();

  return blocks;
}

function EmptyGapBlock({
  rows,
  expanded,
  onToggle,
  design,
  cdsStart,
  cdsEnd,
  selectedId,
  onSelect,
  basesPerRow,
  compact,
}: {
  rows: TranscriptRowData[];
  expanded: boolean;
  onToggle: () => void;
  design: DesignSettings;
  cdsStart: number;
  cdsEnd: number;
  selectedId: string;
  onSelect: (id: string) => void;
  basesPerRow: number;
  compact: boolean;
}) {
  const { t } = useI18n();
  const start = rows[0].start;
  const end = rows[rows.length - 1].end;
  const nt = end - start + 1;

  const foldButton = (
    <button
      type="button"
      onClick={onToggle}
      aria-expanded={expanded}
      className={cn(
        "group flex w-full items-center gap-2 border-y border-dashed border-slate-200 bg-slate-50/80 px-3 text-left transition-colors hover:border-indigo-200 hover:bg-indigo-50/50",
        compact ? "py-2.5" : "py-3",
      )}
    >
      <span className="flex size-5 shrink-0 items-center justify-center rounded-md border border-slate-200 bg-white text-slate-500 group-hover:border-indigo-200 group-hover:text-indigo-600">
        {expanded ? (
          <ChevronUp className="size-3.5" aria-hidden="true" />
        ) : (
          <ChevronDown className="size-3.5" aria-hidden="true" />
        )}
      </span>
      <span
        className={cn(
          "min-w-0 flex-1 font-mono tabular-nums text-slate-500",
          compact ? "text-[10px]" : "text-[11px]",
        )}
      >
        {expanded
          ? t("collapseEmptyGap")
          : t("collapsedEmptyGap", {
              rows: rows.length,
              start,
              end,
              nt,
            })}
      </span>
      {!expanded && (
        <span
          className={cn(
            "shrink-0 font-medium text-indigo-600",
            compact ? "text-[10px]" : "text-[11px]",
          )}
        >
          {t("expandEmptyGap")}
        </span>
      )}
    </button>
  );

  if (!expanded) {
    return <div className="my-1">{foldButton}</div>;
  }

  return (
    <div>
      <div className="my-1">{foldButton}</div>
      {rows.map((row) => (
        <TranscriptRow
          key={row.start}
          row={row}
          design={design}
          cdsStart={cdsStart}
          cdsEnd={cdsEnd}
          selectedId={selectedId}
          onSelect={onSelect}
          basesPerRow={basesPerRow}
          compact={compact}
        />
      ))}
    </div>
  );
}

function TmLegend() {
  const { t } = useI18n();
  const items: Array<{ label: string; band: TmBand }> = [
    { label: "< 10°C", band: "under-10" },
    { label: "< 15°C", band: "under-15" },
    { label: "< 21.5°C", band: "under-21.5" },
    { label: t("functionalSirna"), band: "functional" },
  ];

  return (
    <div className="mt-5 flex flex-wrap items-center gap-x-5 gap-y-2 text-[11px] text-slate-500">
      <span className="font-medium text-slate-700">{t("seedDuplexTm")}</span>
      {items.map(({ label, band }) => (
        <span key={band} className="inline-flex items-center gap-2">
          <span
            className={cn(
              "inline-block size-4 rounded-[4px]",
              tmBandClasses[band],
            )}
          />
          {label}
        </span>
      ))}
      <span className="text-slate-400">{t("offTargetReduced")}</span>
    </div>
  );
}

function TranscriptRow({
  row,
  design,
  cdsStart,
  cdsEnd,
  selectedId,
  onSelect,
  basesPerRow,
  compact,
}: {
  row: TranscriptRowData;
  design: DesignSettings;
  cdsStart: number;
  cdsEnd: number;
  selectedId: string;
  onSelect: (id: string) => void;
  basesPerRow: number;
  compact: boolean;
}) {
  const bases = [...row.sequence];
  const laneCount =
    row.segments.length > 0
      ? Math.max(...row.segments.map(({ lane }) => lane)) + 1
      : 1;
  const ticks = Array.from(
    { length: Math.floor(bases.length / 10) },
    (_, index) => row.start + (index + 1) * 10 - 1,
  );

  return (
    <div
      className={cn(
        "border-t border-slate-100 first:border-t-0 first:pt-1",
        compact ? "py-4" : "py-5",
      )}
    >
      <div
        className={cn(
          "grid",
          compact
            ? "grid-cols-[1fr_3.25rem] gap-1.5"
            : "grid-cols-[1fr_4.5rem] gap-3",
        )}
      >
        <div>
          <div
            className={cn(
              "grid h-5 font-mono tabular-nums text-slate-400",
              compact ? "text-[9px]" : "text-[10px]",
            )}
            style={{
              gridTemplateColumns: `repeat(${basesPerRow}, minmax(0, 1fr))`,
            }}
          >
            {ticks.map((tick, index) => (
              <span
                key={tick}
                className="relative text-center"
                style={{ gridColumn: `${(index + 1) * 10} / span 1` }}
              >
                {tick}
                <span className="absolute left-1/2 top-4 h-1.5 w-px bg-slate-300" />
              </span>
            ))}
          </div>

          <div
            className={cn(
              "mt-1 grid font-mono font-medium leading-4 tracking-[-0.03em]",
              compact ? "text-[9px]" : "text-[10px]",
            )}
            style={{
              gridTemplateColumns: `repeat(${basesPerRow}, minmax(0, 1fr))`,
            }}
          >
            {bases.map((base, index) => {
              const position = row.start + index;
              const inCds = position >= cdsStart && position <= cdsEnd;
              return (
                <span
                  key={position}
                  className={cn(
                    "relative text-center",
                    inCds ? "text-slate-800" : "text-slate-400",
                    position === cdsStart &&
                      "after:absolute after:inset-x-0 after:-bottom-1 after:h-px after:bg-indigo-500",
                  )}
                >
                  {base}
                </span>
              );
            })}
          </div>

          <div
            className="relative mt-2"
            style={{ height: `${Math.max(28, laneCount * 30 - 2)}px` }}
          >
            {row.segments.map((segment) => (
              <CandidateMarker
                key={`${segment.candidate.id}-${row.start}`}
                segment={segment}
                rowStart={row.start}
                basesPerRow={basesPerRow}
                compact={compact}
                design={design}
                selected={segment.candidate.id === selectedId}
                onSelect={onSelect}
              />
            ))}
          </div>
        </div>

        <span
          className={cn(
            "pt-7 text-right font-mono tabular-nums text-slate-400",
            compact ? "text-[9px]" : "text-[10px]",
          )}
        >
          {row.start}–{row.end}
        </span>
      </div>
    </div>
  );
}

function CandidateMarker({
  segment,
  rowStart,
  basesPerRow,
  compact,
  design,
  selected,
  onSelect,
}: {
  segment: CandidateSegment;
  rowStart: number;
  basesPerRow: number;
  compact: boolean;
  design: DesignSettings;
  selected: boolean;
  onSelect: (id: string) => void;
}) {
  const {
    candidate,
    segmentStart,
    segmentEnd,
    lane,
    showDetails,
    continuesFromPreviousRow,
    continuesToNextRow,
  } = segment;
  const left = ((segmentStart - rowStart) / basesPerRow) * 100;
  const width = ((segmentEnd - segmentStart + 1) / basesPerRow) * 100;

  const trigger = (
    <button
      type="button"
      aria-label={`Select ${candidate.id}, positions ${candidate.start} to ${candidate.end}`}
      aria-pressed={selected}
      onClick={() => onSelect(candidate.id)}
      className={cn(
        "absolute top-0 flex h-6 min-w-0 items-center justify-center overflow-visible rounded-md font-mono text-[10px] shadow-none outline-none transition-[box-shadow,transform] focus-visible:ring-2 focus-visible:ring-indigo-500 focus-visible:ring-offset-2",
        showDetails
          ? compact
            ? "gap-1 px-1.5"
            : "gap-1.5 px-2"
          : "px-1",
        tmBandClasses[getTmBand(candidate.seedTm)],
        selected &&
          "z-10 ring-2 ring-indigo-500 ring-offset-2 ring-offset-white",
      )}
      style={{
        left: `${left}%`,
        top: `${lane * 30}px`,
        width: `${width}%`,
      }}
    >
      <span
        className="absolute bottom-full left-0 w-px bg-slate-300"
        style={{ height: `${8 + lane * 30}px` }}
      />
      <span
        className="absolute bottom-full right-0 w-px bg-slate-300"
        style={{ height: `${8 + lane * 30}px` }}
      />
      {showDetails ? (
        <>
          <span className="whitespace-nowrap">
            {candidate.start}–{candidate.end}
          </span>
          <RuleBadges candidate={candidate} compact />
        </>
      ) : (
        <span
          aria-hidden="true"
          className="text-[12px] font-semibold leading-none"
        >
          {continuesFromPreviousRow ? "←" : ""}
          {continuesToNextRow ? "→" : ""}
        </span>
      )}
    </button>
  );

  const details = (
    <CandidateDetailTip candidate={candidate} design={design} />
  );

  if (compact) {
    return (
      <Popover>
        <PopoverTrigger asChild>{trigger}</PopoverTrigger>
        <PopoverContent side="top" align="center">
          {details}
        </PopoverContent>
      </Popover>
    );
  }

  return (
    <HoverCard openDelay={180} closeDelay={120}>
      <HoverCardTrigger asChild>{trigger}</HoverCardTrigger>
      <HoverCardContent side="top" align="center">
        {details}
      </HoverCardContent>
    </HoverCard>
  );
}

function getSegmentsForRow(
  candidates: SirnaCandidate[],
  rowStart: number,
  rowEnd: number,
  basesPerRow: number,
): CandidateSegment[] {
  const segments = candidates
    .filter(({ start, end }) => start <= rowEnd && end >= rowStart)
    .map((candidate) => {
      const segmentStart = Math.max(candidate.start, rowStart);
      const segmentEnd = Math.min(candidate.end, rowEnd);

      return {
        candidate,
        segmentStart,
        segmentEnd,
        showDetails: getPrimaryRowStart(candidate, basesPerRow) === rowStart,
        continuesFromPreviousRow: candidate.start < rowStart,
        continuesToNextRow: candidate.end > rowEnd,
      };
    })
    .sort(
      (a, b) =>
        a.segmentStart - b.segmentStart || a.segmentEnd - b.segmentEnd,
    );

  const laneEnds: number[] = [];

  return segments.map((segment) => {
    let lane = laneEnds.findIndex(
      (occupiedUntil) => occupiedUntil < segment.segmentStart,
    );

    if (lane === -1) {
      lane = laneEnds.length;
      laneEnds.push(segment.segmentEnd);
    } else {
      laneEnds[lane] = segment.segmentEnd;
    }

    return { ...segment, lane };
  });
}

function getPrimaryRowStart(
  candidate: SirnaCandidate,
  basesPerRow: number,
): number {
  const firstRowStart =
    Math.floor((candidate.start - 1) / basesPerRow) * basesPerRow + 1;
  const lastRowStart =
    Math.floor((candidate.end - 1) / basesPerRow) * basesPerRow + 1;

  let primaryRowStart = firstRowStart;
  let longestSegment = 0;

  for (
    let currentRowStart = firstRowStart;
    currentRowStart <= lastRowStart;
    currentRowStart += basesPerRow
  ) {
    const currentRowEnd = currentRowStart + basesPerRow - 1;
    const segmentStart = Math.max(candidate.start, currentRowStart);
    const segmentEnd = Math.min(candidate.end, currentRowEnd);
    const segmentLength = segmentEnd - segmentStart + 1;

    if (segmentLength > longestSegment) {
      longestSegment = segmentLength;
      primaryRowStart = currentRowStart;
    }
  }

  return primaryRowStart;
}
