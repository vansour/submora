export interface DraftLinkStats {
  normalized_count: number;
  blank_count: number;
  duplicate_count: number;
  invalid_count: number;
  first_invalid: string | null;
  normalized_preview: string[];
}

const MAX_URL_LENGTH = 2048;

export function isValidSourceUrl(input: string): boolean {
  const trimmed = input.trim();
  if (trimmed === "" || trimmed.length > MAX_URL_LENGTH) {
    return false;
  }

  let parsed: URL;
  try {
    parsed = new URL(trimmed);
  } catch {
    return false;
  }

  return (parsed.protocol === "http:" || parsed.protocol === "https:") && parsed.hostname !== "";
}

export function parseLinks(linksText: string): string[] {
  return linksText
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

export function normalizeLinksPreserveOrder(links: string[]): string[] {
  const seen = new Set<string>();
  const normalized: string[] = [];

  for (const raw of links) {
    const trimmed = raw.trim();
    if (trimmed === "" || !isValidSourceUrl(trimmed) || seen.has(trimmed)) {
      continue;
    }

    seen.add(trimmed);
    normalized.push(trimmed);
  }

  return normalized;
}

export function analyzeLinks(linksText: string, previewLimit: number): DraftLinkStats {
  const rawLines = linksText === "" ? [] : linksText.split("\n");
  const seen = new Set<string>();
  const normalizedPreview: string[] = [];

  let blankCount = 0;
  let duplicateCount = 0;
  let invalidCount = 0;
  let firstInvalid: string | null = null;

  for (const rawLine of rawLines) {
    const trimmed = rawLine.trim();
    if (trimmed === "") {
      blankCount += 1;
      continue;
    }

    if (!isValidSourceUrl(trimmed)) {
      invalidCount += 1;
      if (firstInvalid === null) {
        firstInvalid = trimmed;
      }
      continue;
    }

    if (seen.has(trimmed)) {
      duplicateCount += 1;
      continue;
    }

    seen.add(trimmed);
    if (normalizedPreview.length < previewLimit) {
      normalizedPreview.push(trimmed);
    }
  }

  return {
    normalized_count: seen.size,
    blank_count: blankCount,
    duplicate_count: duplicateCount,
    invalid_count: invalidCount,
    first_invalid: firstInvalid,
    normalized_preview: normalizedPreview,
  };
}
