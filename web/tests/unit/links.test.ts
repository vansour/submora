import {
  analyzeLinks,
  normalizeLinksPreserveOrder,
  parseLinks,
} from "@/utils/links";

describe("links utils", () => {
  it("analyzes blanks, duplicates, invalid entries, and preview order", () => {
    const stats = analyzeLinks(
      [
        "https://one.example/feed",
        "",
        "https://one.example/feed",
        "notaurl",
        "https://two.example/feed",
        "   ",
      ].join("\n"),
      4,
    );

    expect(stats.normalized_count).toBe(2);
    expect(stats.blank_count).toBe(2);
    expect(stats.duplicate_count).toBe(1);
    expect(stats.invalid_count).toBe(1);
    expect(stats.first_invalid).toBe("notaurl");
    expect(stats.normalized_preview).toEqual([
      "https://one.example/feed",
      "https://two.example/feed",
    ]);
  });

  it("parses and normalizes links while preserving first-seen order", () => {
    const parsed = parseLinks(
      " https://one.example/feed \n\nhttps://two.example/feed\nhttps://one.example/feed ",
    );
    expect(parsed).toEqual([
      "https://one.example/feed",
      "https://two.example/feed",
      "https://one.example/feed",
    ]);

    expect(normalizeLinksPreserveOrder(parsed)).toEqual([
      "https://one.example/feed",
      "https://two.example/feed",
    ]);
  });
});
