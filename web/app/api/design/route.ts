import { NextResponse } from "next/server";
import { validateDesignInput, type DesignInput } from "@/lib/design-input";
import { toSidirectFormBody } from "@/lib/sidirect-form";
import { parseSidirectHtml } from "@/lib/sidirect-parse";

const SIDIRECT_DESIGN_URL = "https://sidirect2.rnai.jp/design.cgi";
const REQUEST_TIMEOUT_MS = 30_000;

export async function POST(request: Request) {
  let input: DesignInput;
  try {
    input = (await request.json()) as DesignInput;
  } catch {
    return NextResponse.json({ error: "Request body must be JSON." }, { status: 400 });
  }

  const validationError = validateDesignInput(input);
  if (validationError) {
    return NextResponse.json({ error: validationError }, { status: 400 });
  }

  let html: string;
  try {
    const upstream = await fetch(SIDIRECT_DESIGN_URL, {
      method: "POST",
      headers: {
        Accept: "text/html,application/xhtml+xml",
        "Content-Type": "application/x-www-form-urlencoded",
        Referer: "https://sidirect2.rnai.jp/",
        "User-Agent":
          "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
      },
      body: toSidirectFormBody(input).toString(),
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    });

    if (!upstream.ok) {
      return NextResponse.json(
        { error: `siDirect returned HTTP ${upstream.status}.` },
        { status: 502 },
      );
    }

    html = await upstream.text();
  } catch (error) {
    const message =
      error instanceof Error && error.name === "TimeoutError"
        ? "siDirect timed out after 30 seconds."
        : "Could not reach siDirect.";
    return NextResponse.json({ error: message }, { status: 502 });
  }

  try {
    return NextResponse.json({ result: parseSidirectHtml(html, input) });
  } catch (error) {
    const message =
      error instanceof Error ? error.message : "Failed to parse siDirect HTML.";
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
