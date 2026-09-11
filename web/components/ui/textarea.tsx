import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

function Textarea({ className, ...props }: ComponentProps<"textarea">) {
  return (
    <textarea
      data-slot="textarea"
      className={cn(
        "min-h-36 w-full resize-y rounded-md border border-slate-200 bg-white px-3 py-2.5 text-sm text-slate-900 outline-none transition-colors placeholder:text-slate-400 focus-visible:border-indigo-300 focus-visible:ring-2 focus-visible:ring-indigo-500 disabled:cursor-not-allowed disabled:bg-slate-50 disabled:opacity-60",
        className,
      )}
      {...props}
    />
  );
}

export { Textarea };
