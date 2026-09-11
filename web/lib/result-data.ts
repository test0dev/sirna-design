import resultJson from "@/mock/ifnb1.json";
import type { SirnaResult } from "@/lib/sirna-types";

function validateResult(value: SirnaResult): SirnaResult {
  const { transcript, cds, design, sirnas } = value;

  if (transcript.sequence.length !== transcript.length) {
    throw new Error("Mock transcript length does not match its sequence.");
  }
  if (cds.start < 1 || cds.end > transcript.length || cds.start > cds.end) {
    throw new Error("Mock CDS coordinates are outside the transcript.");
  }
  if (
    design.targetRange.from < 1 ||
    design.targetRange.to > transcript.length
  ) {
    throw new Error("Mock target range is outside the transcript.");
  }
  if (
    sirnas.some(
      ({ start, end }) =>
        start < 1 || end > transcript.length || start > end,
    )
  ) {
    throw new Error("A mock siRNA coordinate is outside the transcript.");
  }

  return value;
}

export const resultData = validateResult(resultJson as SirnaResult);
