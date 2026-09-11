"use client";

import { useI18n } from "@/components/i18n-provider";
import { cn } from "@/lib/utils";

export function LanguageSwitch() {
  const { locale, setLocale, t } = useI18n();

  return (
    <div
      role="group"
      aria-label={t("language")}
      className="inline-flex items-center rounded-md border border-slate-200 p-0.5 text-xs"
    >
      <button
        type="button"
        onClick={() => setLocale("zh")}
        className={cn(
          "rounded px-2 py-1 font-medium outline-none focus-visible:ring-2 focus-visible:ring-indigo-500",
          locale === "zh"
            ? "bg-indigo-50 text-indigo-700"
            : "text-slate-500 hover:text-slate-800",
        )}
      >
        {t("langZh")}
      </button>
      <button
        type="button"
        onClick={() => setLocale("en")}
        className={cn(
          "rounded px-2 py-1 font-medium outline-none focus-visible:ring-2 focus-visible:ring-indigo-500",
          locale === "en"
            ? "bg-indigo-50 text-indigo-700"
            : "text-slate-500 hover:text-slate-800",
        )}
      >
        {t("langEn")}
      </button>
    </div>
  );
}
