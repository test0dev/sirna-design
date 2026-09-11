const DEFAULT_API_BASE = "http://127.0.0.1:8080";

/** Rust API origin. `NEXT_PUBLIC_API_BASE` overrides the local default. */
export function apiBase(): string {
  const raw = process.env.NEXT_PUBLIC_API_BASE?.trim();
  return (raw && raw.length > 0 ? raw : DEFAULT_API_BASE).replace(/\/+$/, "");
}

/** Absolute URL for a Rust API path (`/v1/design`, `health`, …). */
export function apiUrl(path: string): string {
  const suffix = path.startsWith("/") ? path : `/${path}`;
  return `${apiBase()}${suffix}`;
}
