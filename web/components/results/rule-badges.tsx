import type { DesignRuleName, SirnaCandidate } from "@/lib/sirna-types";
import { cn } from "@/lib/utils";

const rules: Array<{
  key: DesignRuleName;
  short: string;
  className: string;
}> = [
  { key: "Ui-Tei", short: "U", className: "bg-rose-50 text-rose-700" },
  { key: "Reynolds", short: "R", className: "bg-sky-50 text-sky-700" },
  {
    key: "Amarzguioui",
    short: "A",
    className: "bg-emerald-50 text-emerald-700",
  },
];

export function RuleBadges({
  candidate,
  compact = false,
}: {
  candidate: SirnaCandidate;
  compact?: boolean;
}) {
  return (
    <span className="inline-flex items-center gap-1">
      {rules.map(({ key, short, className }) =>
        candidate.rules[key] ? (
          <span
            key={key}
            title={key}
            className={cn(
              "inline-flex items-center justify-center rounded-[4px] font-semibold",
              compact ? "size-4 text-[9px]" : "size-5 text-[10px]",
              className,
            )}
          >
            {short}
          </span>
        ) : null,
      )}
    </span>
  );
}
