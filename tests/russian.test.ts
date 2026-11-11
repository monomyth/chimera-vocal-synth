/**
 * The wasm lyric call sings Cyrillic without the English dictionary, and leaves an English
 * line on that dictionary.
 */
import { describe, expect, test } from "bun:test";
import { initEngine } from "./harness.ts";

await initEngine();
const { lyricsToSyllables } = await import("../src/wasm/engine.ts");

describe("Russian lyrics", () => {
  test("Cyrillic is sung, English is not retokenized into it", () => {
    const ru = lyricsToSyllables("ма́-ма");
    expect(ru.map((s) => s.nucleus.ipa)).toEqual(["ä", "ɐ"]);
    expect(ru.map((s) => s.text)).toEqual(["ма́", "ма"]);
    const en = lyricsToSyllables("la la");
    expect(en).toHaveLength(2);
    expect(en.every((s) => s.nucleus.ipa !== "ɐ")).toBe(true);
    expect(lyricsToSyllables("ма́-ма", "en")).toHaveLength(0);
  });
});
