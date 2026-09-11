import type { Metadata } from "next";
import type { ReactNode } from "react";
import { I18nProvider } from "@/components/i18n-provider";
import "@/app/globals.css";

export const metadata: Metadata = {
  title: "siRNA 设计",
  description: "siRNA 设计输入与候选可视化",
};

export default function RootLayout({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <html lang="en" data-scroll-behavior="smooth">
      <body>
        <I18nProvider>{children}</I18nProvider>
      </body>
    </html>
  );
}
