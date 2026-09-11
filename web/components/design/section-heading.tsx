import type { ReactNode } from "react";

export function SectionHeading({
  id,
  eyebrow,
  title,
  description,
  action,
}: {
  id?: string;
  eyebrow: string;
  title: string;
  description?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="flex flex-col justify-between gap-2 sm:flex-row sm:items-end">
      <div>
        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-indigo-600">
          {eyebrow}
        </p>
        <h2
          id={id}
          className="mt-1 text-lg font-semibold tracking-tight text-slate-950"
        >
          {title}
        </h2>
        {description ? (
          <p className="mt-1 max-w-2xl text-xs leading-5 text-slate-500">
            {description}
          </p>
        ) : null}
      </div>
      {action}
    </div>
  );
}
