import { CircleHelp } from "lucide-react";
import type { ReactNode } from "react";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";

export function HelpTip({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          type="button"
          aria-label={label}
          className="inline-flex size-5 items-center justify-center rounded-full text-slate-400 outline-none hover:bg-slate-100 hover:text-slate-600 focus-visible:ring-2 focus-visible:ring-indigo-500"
        >
          <CircleHelp className="size-3.5" />
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-[320px] text-xs leading-5 text-slate-600">
        {children}
      </PopoverContent>
    </Popover>
  );
}
