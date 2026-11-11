//! Lyrics → sung syllables. One path for every language: a [`WordSource`] turns graphemes into
//! inventory IPA, and the maximal-onset split uses that language's onset clusters. English is the
//! default source (CMUdict, unreduced: sung function words keep full vowels) with English onsets;
//! its ASCII tokenization and its clusters are unchanged. A language may override
//! [`WordSource::word_hyphenated`] when a typed hyphen is a syllable break but stress and
//! reduction need the whole word (Russian).
//! Typed hyphens force syllable breaks; a lone "-" (or "~") holds the previous vowel onto the
//! next note (melisma), as in vocaloid editors. A combining acute after a Cyrillic vowel stays
//! inside the word so a stress mark is not a break.

use crate::anatomy::human::BodyMix;
use super::english::{EnglishG2P, js_is_space};
use super::planner::is_vowel;
use super::russian::RussianG2P;
use super::{Segment, Syllable};
use serde::{Deserialize, Serialize};

// English onsets (IPA), longest first matters only for the search below
#[rustfmt::skip]
const ONSETS: &[&str] = &[
    "p", "b", "t", "d", "k", "g", "f", "v", "θ", "ð", "s", "z", "ʃ", "ʒ", "h", "m", "n", "l", "ɹ", "w", "j", "tʃ", "dʒ",
    "pl", "bl", "kl", "gl", "fl", "sl", "pɹ", "bɹ", "tɹ", "dɹ", "kɹ", "gɹ", "fɹ", "θɹ", "ʃɹ",
    "sp", "st", "sk", "sm", "sn", "sw", "sf", "tw", "kw", "dw", "gw", "θw",
    "pj", "bj", "kj", "gj", "mj", "nj", "fj", "vj", "hj", "lj",
    "spl", "spɹ", "stɹ", "skɹ", "skw", "skj", "spj",
];

/// One syllable of a word, before it gets its text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyllableParts {
    pub onset: Vec<Segment>,
    pub nucleus: Segment,
    pub coda: Vec<Segment>,
}

/// Split one word's segments into onset / nucleus / coda syllables (English onsets).
pub fn syllabify_segments(segs: &[Segment]) -> Vec<SyllableParts> {
    syllabify_with(segs, ONSETS)
}

/// [`syllabify_segments`] with a language's onset list. English clusters are at most three
/// segments, so trying a fourth length changes nothing for [`ONSETS`].
pub(crate) fn syllabify_with(segs: &[Segment], onsets: &[&str]) -> Vec<SyllableParts> {
    let vi: Vec<usize> = segs.iter().enumerate().filter(|(_, s)| is_vowel(&s.ipa, BodyMix::MALE)).map(|(i, _)| i).collect();
    if vi.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut start = 0;
    for (n, &v) in vi.iter().enumerate() {
        let mut end = segs.len(); // exclusive end of this syllable
        if let Some(&next) = vi.get(n + 1) {
            // consonants between this vowel and the next: the longest legal onset goes right
            let between = &segs[v + 1..next];
            let mut k = 0;
            for len in (1..=4.min(between.len())).rev() {
                let cluster: String = between[between.len() - len..].iter().map(|s| s.ipa.as_str()).collect();
                if onsets.contains(&cluster.as_str()) {
                    k = len;
                    break;
                }
            }
            end = next - k;
        }
        out.push(SyllableParts { onset: segs[start..v].to_vec(), nucleus: segs[v].clone(), coda: segs[v + 1..end].to_vec() });
        start = end;
    }
    out
}

// The spelling is indexed in UTF-16 code units, as the JS string was.
fn is_cyr_vowel(c: u16) -> bool {
    // а е ё и о у ы э ю я, and the capitals. Cyrillic is in the BMP, one UTF-16 unit.
    matches!(c, 0x0430 | 0x0435 | 0x0451 | 0x0438 | 0x043E | 0x0443 | 0x044B | 0x044D | 0x044E | 0x044F
        | 0x0410 | 0x0415 | 0x0401 | 0x0418 | 0x041E | 0x0423 | 0x042B | 0x042D | 0x042E | 0x042F)
}
fn is_spell_vowel(c: u16) -> bool {
    "aeiouy".bytes().any(|v| v as u16 == c) || is_cyr_vowel(c)
}
fn is_spell_vowel_ci(c: u16) -> bool {
    "aeiouyAEIOUY".bytes().any(|v| v as u16 == c) || is_cyr_vowel(c)
}
fn eq_ci(c: u16, lower: u8) -> bool {
    c == lower as u16 || c == lower.to_ascii_uppercase() as u16
}
/// `word.slice(a, b)` (indices clamped to the word)
fn slice(w: &[u16], a: usize, b: usize) -> String {
    let (a, b) = (a.min(w.len()), b.min(w.len()));
    if a >= b { String::new() } else { String::from_utf16_lossy(&w[a..b]) }
}

/// Split a written word into n chunks at its vowel groups (approximate hyphenation).
pub fn split_spelling(word: &str, n: usize) -> Vec<String> {
    if n <= 1 {
        return vec![word.to_string()];
    }
    let w: Vec<u16> = word.encode_utf16().collect();
    let lower: Vec<u16> = word.to_lowercase().encode_utf16().collect();
    // /[aeiouy]+/g on the lowercased word
    let mut groups: Vec<[usize; 2]> = Vec::new();
    let mut i = 0;
    while i < lower.len() {
        if is_spell_vowel(lower[i]) {
            let s = i;
            while i < lower.len() && is_spell_vowel(lower[i]) {
                i += 1;
            }
            groups.push([s, i]);
        } else {
            i += 1;
        }
    }
    let len = w.len();
    // /[^aeiouy]e$/i and /[^aeiouyl]le$/i
    let silent_e = len >= 2 && eq_ci(w[len - 1], b'e') && !is_spell_vowel_ci(w[len - 2]);
    let le = len >= 3 && eq_ci(w[len - 1], b'e') && eq_ci(w[len - 2], b'l') && !is_spell_vowel_ci(w[len - 3]) && !eq_ci(w[len - 3], b'l');
    // a final silent e ("like") is not a syllable, but consonant + "le" is ("twin-kle", "lit-tle")
    if groups.len() > 1 && silent_e && !le && groups[groups.len() - 1][0] + 1 == len {
        groups.pop();
    }
    if groups.len() < n {
        // fewer written vowels than sung syllables (e.g. "fire"): hold the word, then continue marks
        let mut out = vec![word.to_string()];
        out.extend(std::iter::repeat_n("~".to_string(), n - 1));
        return out;
    }
    while groups.len() > n {
        let last = groups.pop().unwrap();
        let k = groups.len() - 1;
        groups[k][1] = last[1];
    }
    let mut cuts: Vec<usize> = vec![0];
    for i in 0..groups.len() - 1 {
        let (a, b) = (groups[i][1], groups[i + 1][0]); // consonant letters a..b between vowel groups
        // a digraph stays together on the right ("any-where", "fa-ther"); otherwise one consonant
        // goes right and longer runs split in the middle ("lit-tle", "won-der")
        let gap = b - a;
        let dig = gap >= 2 && {
            let d = [w.get(b - 2).copied(), w.get(b - 1).copied()];
            ["th", "sh", "ch", "ph", "wh", "ck", "gh"].iter().any(|p| {
                let p = p.as_bytes();
                d[0].is_some_and(|c| eq_ci(c, p[0])) && d[1].is_some_and(|c| eq_ci(c, p[1]))
            })
        };
        cuts.push(if dig { b - 2 } else { a + if gap <= 1 { 0 } else { gap / 2 } });
    }
    cuts.push(len);
    (0..cuts.len() - 1).map(|i| slice(&w, cuts[i], cuts[i + 1])).collect()
}

/// Where lyrics get their pronunciations. English (`EnglishG2P`) and Russian (`RussianG2P`) are
/// the two sources; a test can pass any `Fn(&str, bool) -> Vec<Segment>`.
pub trait WordSource {
    fn word(&self, w: &str, reduce: bool) -> Vec<Segment>;

    /// Onset clusters the maximal-onset split may assign to the next syllable. The default is
    /// the English list, so a source that does not care (and every `Fn`) syllabifies as English.
    fn onsets(&self) -> &'static [&'static str] {
        ONSETS
    }

    /// Pronounce a hyphenated word as one word, then group the syllables to match `parts`.
    /// `None` means each piece is its own dictionary word (English: "ti-ger" looks up "ti" and
    /// "ger"). Russian returns `Some`: the hyphens are the sung breaks, but stress and reduction
    /// are read from the whole word.
    fn word_hyphenated(&self, _parts: &[&str], _reduce: bool) -> Option<Vec<Vec<SyllableParts>>> {
        None
    }
}

impl<F: Fn(&str, bool) -> Vec<Segment>> WordSource for F {
    fn word(&self, w: &str, reduce: bool) -> Vec<Segment> {
        self(w, reduce)
    }
}

fn is_lyric_punct(c: char) -> bool {
    matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | '\n')
}
pub(crate) fn is_cyrillic(c: char) -> bool {
    ('\u{0400}'..='\u{04FF}').contains(&c)
}
fn is_word_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '\'' || is_cyrillic(c)
}
/// ASCII words continue on letters, apostrophes and hyphens, as before. A combining acute is
/// part of the word only after a Cyrillic letter, so an accented Latin word still breaks where
/// it used to.
fn continues_word(c: char, prev: char) -> bool {
    is_word_start(c) || c == '-' || (c == '\u{0301}' && is_cyrillic(prev))
}

/// `text.match(/[A-Za-z'][A-Za-z'\-]*|(?<=\s|^)[~-](?=\s|$)|[.,;:!?\n]/g)`
fn lyric_tokens(text: &str) -> Vec<String> {
    let s: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if is_word_start(c) {
            let start = i;
            i += 1;
            while i < s.len() && continues_word(s[i], s[i - 1]) {
                i += 1;
            }
            out.push(s[start..i].iter().collect());
            continue;
        }
        let melisma = (c == '~' || c == '-') && (i == 0 || js_is_space(s[i - 1])) && (i + 1 == s.len() || js_is_space(s[i + 1]));
        if melisma || is_lyric_punct(c) {
            out.push(c.to_string());
        }
        i += 1;
    }
    out
}

/// Lyrics text → syllables (needs the pronouncing dictionary for real words).
pub fn lyrics_to_syllables(text: &str, g2p: &impl WordSource) -> Vec<Syllable> {
    let mut out: Vec<Syllable> = Vec::new();
    // words, melisma marks and phrase punctuation, in order
    let tokens = lyric_tokens(&text.replace('’', "'"));
    for tok in &tokens {
        if tok.chars().count() == 1 && tok.chars().all(is_lyric_punct) {
            if let Some(last) = out.last_mut() {
                last.phrase_end = true;
            }
            continue;
        }
        if tok == "-" || tok == "~" {
            if let Some(prev) = out.last() {
                let s = Syllable { text: "~".into(), onset: vec![], nucleus: prev.nucleus.clone(), coda: vec![], melisma: Some(true), word_end: prev.word_end, phrase_end: false };
                out.push(s);
            }
            continue;
        }
        // typed hyphens split syllables explicitly ("ti-ger"); otherwise the dictionary decides.
        // A language that needs the whole word (Russian stress) supplies the groups itself.
        let parts: Vec<&str> = tok.split('-').filter(|p| !p.is_empty()).collect();
        let mut syls: Vec<Syllable> = Vec::new();
        let grouped = if parts.len() > 1 { g2p.word_hyphenated(&parts, false) } else { None };
        if let Some(groups) = grouped {
            for (part, sy) in parts.iter().zip(groups) {
                push_spelled(&mut syls, part, sy, true);
            }
        } else {
            for part in &parts {
                let sy = syllabify_with(&g2p.word(&part.to_lowercase(), false), g2p.onsets());
                push_spelled(&mut syls, part, sy, parts.len() > 1);
            }
        }
        if let Some(last) = syls.last_mut() {
            last.word_end = true;
        }
        out.append(&mut syls);
    }
    out
}

fn push_spelled(syls: &mut Vec<Syllable>, part: &str, sy: Vec<SyllableParts>, hyphenated: bool) {
    let spell: Vec<String> = if hyphenated {
        let mut v = vec![part.to_string()];
        v.extend(std::iter::repeat_n("~".to_string(), sy.len().saturating_sub(1)));
        v
    } else {
        split_spelling(part, sy.len())
    };
    for (i, s) in sy.into_iter().enumerate() {
        syls.push(Syllable { text: spell.get(i).cloned().unwrap_or_else(|| "~".into()), onset: s.onset, nucleus: s.nucleus, coda: s.coda, melisma: None, word_end: false, phrase_end: false });
    }
}

/// Which lyric front end sings a line. English is the default; Russian is Cyrillic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LyricLang {
    En,
    Ru,
}

/// `requested` is the melody's `language` field: `en` / `ru`, or absent. Absent selects Russian
/// when the line contains any Cyrillic letter, otherwise English. An explicit `en` stays English
/// even if the line quotes Cyrillic (those letters produce no English syllable, as before).
pub fn lyric_language(text: &str, requested: Option<&str>) -> Result<LyricLang, String> {
    let req = requested.map(str::trim).filter(|s| !s.is_empty());
    match req.map(|s| s.to_ascii_lowercase()).as_deref() {
        None => Ok(if text.chars().any(is_cyrillic) { LyricLang::Ru } else { LyricLang::En }),
        Some("en" | "english") => Ok(LyricLang::En),
        Some("ru" | "rus" | "russian") => Ok(LyricLang::Ru),
        Some(other) => Err(format!("unknown language \"{other}\" (expected en or ru)")),
    }
}

/// Lyrics through the shared front end. `english` is ignored for Russian (no dictionary).
pub fn sing_lyrics(text: &str, language: Option<&str>, english: &EnglishG2P) -> Result<Vec<Syllable>, String> {
    Ok(match lyric_language(text, language)? {
        LyricLang::En => lyrics_to_syllables(text, english),
        LyricLang::Ru => lyrics_to_syllables(text, &RussianG2P),
    })
}
