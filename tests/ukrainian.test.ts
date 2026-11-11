/**
 * Ukrainian uses the same lyric call as Russian. і/ї/є/ґ select it when language is omitted.
 */
import { describe, expect, test } from "bun:test";
import { initEngine } from "./harness.ts";

await initEngine();
const { lyricsToSyllables } = await import("../src/wasm/engine.ts");

const ANTHEM = "Ще не вмерла України і слава, і воля";

describe("Ukrainian lyrics", () => {
  test("the anthem line is fourteen full vowels, and Russian still reduces", () => {
    const uk = lyricsToSyllables(ANTHEM, "uk");
    expect(uk.map((s) => s.nucleus.ipa)).toEqual([
      "ɛ", "ɛ", "ɛ", "a", "u", "a", "i", "ɪ", "i", "a", "a", "i", "ɔ", "a",
    ]);
    expect(lyricsToSyllables(ANTHEM).map((s) => s.nucleus.ipa)).toEqual(
      uk.map((s) => s.nucleus.ipa),
    );
    expect(lyricsToSyllables("ма́-ма").map((s) => s.nucleus.ipa)).toEqual(["ä", "ɐ"]);
    expect(lyricsToSyllables("не", "uk")[0].nucleus.ipa).toBe("ɛ");
  });
});
