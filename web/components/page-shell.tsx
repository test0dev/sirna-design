import type { ReactNode } from "react";
import { LanguageSwitch } from "@/components/language-switch";

export function PageShell({ children }: { children: ReactNode }) {
  return (
    <main className="min-h-screen">
      <div className="mx-auto w-full max-w-[1280px] sm:px-6 sm:py-8 lg:px-10 lg:py-12">
        <div className="mb-3 flex justify-end px-4 sm:px-1">
          <LanguageSwitch />
        </div>
        <div className="bg-white px-4 py-6 sm:rounded-2xl sm:px-8 sm:py-7 sm:shadow-[0_1px_2px_rgba(15,23,42,0.04)] lg:px-10">
          {children}
        </div>
      </div>
    </main>
  );
}
