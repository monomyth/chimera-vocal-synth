//! Ukrainian lyrics → inventory IPA.
//!
//! Vowels keep their full quality whether or not they are stressed. There is no Russian-style
//! reduction to `ɐ` / `ə` / `ɪ`. Stress is still chosen, because the planner shortens an
//! unstressed vowel: an acute on the vowel, then a small authored list, then the last vowel.
//! A clitic (`не`, `і`, `в`, …) stays unstressed. Homographs are left out, and this is not a
//! copied dictionary.
//!
//! `и` is `ɪ` and `і` is `i`. `ї` is always `j` + `i`. `є` / `ю` / `я` are `j` plus the vowel
//! at the start of a word, after a vowel, and after an apostrophe (`з'їзд`, `м'ясо`); after a
//! consonant they palatalize it instead. `е` does not palatalize the consonant before it
//! (`не` is `n ɛ`, not a soft `н`). `г` is `ɦ` and `ґ` is `g`. `щ` is `ʃtʃ`. Final obstruents
//! stay voiced (`народ` keeps `д`).

use super::Segment;
use super::syllables::{SyllableParts, WordSource, syllabify_with};

#[rustfmt::skip]
const ONSETS: &[&str] = &[
    "p", "b", "t", "d", "k", "g", "m", "n", "f", "v", "s", "z", "ʃ", "ʒ", "x", "ɦ",
    "ts", "tʃ", "ʃtʃ", "r", "l", "j", "w",
    "pʲ", "bʲ", "tʲ", "dʲ", "kʲ", "gʲ", "mʲ", "nʲ", "fʲ", "vʲ", "sʲ", "zʲ", "xʲ", "lʲ", "rʲ",
    "pr", "br", "tr", "dr", "kr", "gr", "fr", "vr", "xr",
    "pl", "bl", "kl", "gl", "fl", "sl", "vl",
    "sp", "st", "sk", "sv", "zv", "dv", "tv",
    "pj", "bj", "mj", "fj", "vj", "rj",
    "pʲj", "bʲj", "mʲj", "fʲj", "vʲj", "rʲj",
];

const CLITICS: &[&str] = &[
    "а", "але", "б", "без", "би", "бо", "в", "від", "для", "до", "ж", "же", "з", "зі", "й", "і",
    "із", "на", "над", "не", "ні", "по", "під", "при", "про", "та", "у", "чи", "що",
];

#[derive(Clone, Copy)]
struct Letter {
    ch: char,
    part: usize,
    marked: bool,
    after_apostrophe: bool,
}

struct Unit {
    ipa: &'static str,
    part: usize,
    vowel: bool,
    stress: u8,
    skip: bool,
    assimilate: bool,
    spread: bool,
    underlying_voiced: bool,
}

#[derive(Clone, Copy)]
struct Cons {
    hard: &'static str,
    soft: &'static str,
    glide: bool,
    voiced: bool,
    assimilate: bool,
    spread: bool,
    sonorant: bool,
}

/// Ukrainian grapheme-to-phoneme. `reduce` is accepted so the trait matches English and
/// Russian; vowel quality does not change with it.
#[derive(Clone, Copy, Debug, Default)]
pub struct UkrainianG2P;

impl UkrainianG2P {
    pub fn word(&self, w: &str, _speech: bool) -> Vec<Segment> {
        let flat = w.replace('-', "");
        flatten(&pronounce(&[flat.as_str()]))
    }
}

impl WordSource for UkrainianG2P {
    fn word(&self, w: &str, reduce: bool) -> Vec<Segment> {
        UkrainianG2P::word(self, w, reduce)
    }
    fn onsets(&self) -> &'static [&'static str] {
        ONSETS
    }
    fn word_hyphenated(&self, parts: &[&str], _reduce: bool) -> Option<Vec<Vec<SyllableParts>>> {
        Some(pronounce(parts))
    }
}

fn flatten(groups: &[Vec<SyllableParts>]) -> Vec<Segment> {
    let mut out = Vec::new();
    for g in groups {
        for s in g {
            out.extend(s.onset.iter().cloned());
            out.push(s.nucleus.clone());
            out.extend(s.coda.iter().cloned());
        }
    }
    out
}

fn pronounce(parts: &[&str]) -> Vec<Vec<SyllableParts>> {
    let letters = parse_parts(parts);
    let n = parts.len().max(1);
    if letters.is_empty() {
        return vec![vec![]; n];
    }
    let stressed = stress_marks(&letters, parts);
    let soft = softness(&letters);
    let mut units = Vec::new();
    for i in 0..letters.len() {
        let g = letters[i];
        if is_vowel_letter(g.ch) {
            if inserts_j(&letters, i) {
                units.push(glide(g.part));
            }
            units.push(Unit {
                ipa: vowel_ipa(g.ch),
                part: g.part,
                vowel: true,
                stress: if stressed[i] { 1 } else { 0 },
                skip: false,
                assimilate: false,
                spread: false,
                underlying_voiced: true,
            });
        } else if g.ch == 'щ' {
            units.push(plain("ʃ", g.part, false));
            units.push(plain("tʃ", g.part, false));
        } else if let Some(c) = consonant(g.ch) {
            emit_cons(&mut units, &letters, i, c, soft[i]);
        }
    }
    assimilate(&mut units);
    group(units, n)
}

fn parse_parts(parts: &[&str]) -> Vec<Letter> {
    let mut out: Vec<Letter> = Vec::new();
    for (pi, part) in parts.iter().enumerate() {
        let mut apostrophe = false;
        for c in part.to_lowercase().chars() {
            if c == '\u{0301}' {
                if let Some(last) = out.last_mut()
                    && last.part == pi
                    && is_vowel_letter(last.ch)
                {
                    last.marked = true;
                }
                continue;
            }
            if matches!(c, '\'' | '\u{2019}' | '\u{02BC}') {
                apostrophe = true;
                continue;
            }
            if c == '-' || c == '_' || c == '\u{203F}' || (!is_vowel_letter(c) && consonant(c).is_none() && c != 'ь' && c != 'щ') {
                continue;
            }
            out.push(Letter {
                ch: c,
                part: pi,
                marked: false,
                after_apostrophe: apostrophe,
            });
            apostrophe = false;
        }
    }
    out
}

fn bare(parts: &[&str]) -> String {
    let mut s = String::new();
    for p in parts {
        for c in p.to_lowercase().chars() {
            if !matches!(c, '\u{0301}' | '\'' | '\u{2019}' | '\u{02BC}' | '-' | '_' | '\u{203F}') {
                s.push(c);
            }
        }
    }
    s
}

fn stress_marks(letters: &[Letter], parts: &[&str]) -> Vec<bool> {
    let vowels: Vec<usize> = (0..letters.len()).filter(|&i| is_vowel_letter(letters[i].ch)).collect();
    let mut stressed = vec![false; letters.len()];
    if vowels.iter().any(|&i| letters[i].marked) {
        for i in vowels {
            stressed[i] = letters[i].marked;
        }
    } else if CLITICS.contains(&bare(parts).as_str()) {
        // unstressed, and still a full vowel
    } else if let Some(v) = dict_stress(&bare(parts)) {
        if let Some(&i) = vowels.get(v) {
            stressed[i] = true;
        }
    } else if vowels.len() <= 1 {
        if let Some(i) = vowels.first() {
            stressed[*i] = true;
        }
    } else if let Some(&i) = vowels.last() {
        stressed[i] = true;
    }
    stressed
}

fn softness(letters: &[Letter]) -> Vec<bool> {
    let mut soft = vec![false; letters.len()];
    for i in 0..letters.len() {
        if consonant(letters[i].ch).is_none() || always_hard(letters[i].ch) || letters[i].ch == 'г' {
            continue;
        }
        soft[i] = match letters.get(i + 1) {
            Some(n) if n.ch == 'ь' => true,
            Some(n) if n.ch == 'і' && !n.after_apostrophe => true,
            Some(n) if is_iotated(n.ch) && !inserts_j(letters, i + 1) => true,
            _ => false,
        };
    }
    soft
}

fn emit_cons(units: &mut Vec<Unit>, letters: &[Letter], i: usize, c: Cons, soft: bool) {
    let part = letters[i].part;
    let front = letters.get(i + 1).is_some_and(|n| matches!(n.ch, 'і' | 'и'));
    let vowel_j = ((i + 1)..letters.len()).find(|&k| is_vowel_letter(letters[k].ch)).is_some_and(|v| inserts_j(letters, v));
    let ipa = if soft { c.soft } else { c.hard };
    units.push(cons_unit(ipa, part, c));
    if c.glide && soft && !front && !vowel_j {
        units.push(glide(part));
    }
}

fn cons_unit(ipa: &'static str, part: usize, c: Cons) -> Unit {
    Unit {
        ipa,
        part,
        vowel: false,
        stress: 0,
        skip: c.sonorant || !c.spread,
        assimilate: c.assimilate,
        spread: c.spread,
        underlying_voiced: c.voiced,
    }
}

fn plain(ipa: &'static str, part: usize, voiced: bool) -> Unit {
    Unit {
        ipa,
        part,
        vowel: false,
        stress: 0,
        skip: false,
        assimilate: false,
        spread: true,
        underlying_voiced: voiced,
    }
}

fn glide(part: usize) -> Unit {
    Unit {
        ipa: "j",
        part,
        vowel: false,
        stress: 0,
        skip: true,
        assimilate: false,
        spread: false,
        underlying_voiced: true,
    }
}

/// Regressive voicing only. Ukrainian does not devoice a word-final obstruent, so the chain
/// starts with no forced voicelessness.
fn assimilate(units: &mut [Unit]) {
    let mut force: Option<bool> = None;
    for i in (0..units.len()).rev() {
        if units[i].vowel {
            force = None;
            continue;
        }
        if units[i].skip && !units[i].assimilate {
            continue;
        }
        if units[i].assimilate {
            let eff = force.unwrap_or(units[i].underlying_voiced);
            if force.is_some() {
                units[i].ipa = voiced_ipa(units[i].ipa, eff);
            }
            if units[i].spread {
                force = Some(eff);
            }
            continue;
        }
        if units[i].spread {
            force = Some(units[i].underlying_voiced);
        }
    }
}

fn voiced_ipa(ipa: &'static str, voiced: bool) -> &'static str {
    match (ipa, voiced) {
        ("p", true) => "b",
        ("b", false) => "p",
        ("f", true) => "v",
        ("v", false) => "f",
        ("t", true) => "d",
        ("d", false) => "t",
        ("s", true) => "z",
        ("z", false) => "s",
        ("ʃ", true) => "ʒ",
        ("ʒ", false) => "ʃ",
        ("k", true) => "g",
        ("g", false) => "k",
        ("x", true) => "ɦ",
        ("ɦ", false) => "x",
        ("pʲ", true) => "bʲ",
        ("bʲ", false) => "pʲ",
        ("fʲ", true) => "vʲ",
        ("vʲ", false) => "fʲ",
        ("tʲ", true) => "dʲ",
        ("dʲ", false) => "tʲ",
        ("sʲ", true) => "zʲ",
        ("zʲ", false) => "sʲ",
        ("kʲ", true) => "gʲ",
        ("gʲ", false) => "kʲ",
        (other, _) => other,
    }
}

fn group(units: Vec<Unit>, n: usize) -> Vec<Vec<SyllableParts>> {
    let mut buckets: Vec<Vec<Segment>> = vec![Vec::new(); n];
    for u in units {
        let seg = Segment { ipa: u.ipa.to_string(), stress: Some(u.stress), ..Default::default() };
        if let Some(b) = buckets.get_mut(u.part) {
            b.push(seg);
        }
    }
    buckets
        .into_iter()
        .map(|segs| {
            let vowels: Vec<usize> = segs.iter().enumerate().filter(|(_, s)| is_vowel_ipa(&s.ipa)).map(|(i, _)| i).collect();
            if vowels.is_empty() {
                return vec![SyllableParts { onset: segs, nucleus: Segment::new("ə"), coda: vec![] }];
            }
            if vowels.len() == 1 {
                let v = vowels[0];
                return vec![SyllableParts { onset: segs[..v].to_vec(), nucleus: segs[v].clone(), coda: segs[v + 1..].to_vec() }];
            }
            syllabify_with(&segs, ONSETS)
        })
        .collect()
}

fn is_vowel_ipa(ipa: &str) -> bool {
    matches!(ipa, "a" | "ɛ" | "ɔ" | "u" | "i" | "ɪ" | "ə")
}

fn vowel_ipa(ch: char) -> &'static str {
    match ch {
        'е' | 'є' => "ɛ",
        'и' => "ɪ",
        'і' | 'ї' => "i",
        'о' => "ɔ",
        'у' | 'ю' => "u",
        _ => "a",
    }
}

fn inserts_j(letters: &[Letter], i: usize) -> bool {
    let c = letters[i].ch;
    if c == 'ї' {
        return true;
    }
    if !is_iotated(c) {
        return false;
    }
    if letters[i].after_apostrophe {
        return true;
    }
    match prev_letter(letters, i) {
        None => true,
        Some(p) if is_vowel_letter(p) => true,
        _ => false,
    }
}

fn prev_letter(letters: &[Letter], i: usize) -> Option<char> {
    i.checked_sub(1).map(|k| letters[k].ch)
}

fn is_vowel_letter(c: char) -> bool {
    matches!(c, 'а' | 'е' | 'и' | 'і' | 'о' | 'у' | 'ю' | 'я' | 'є' | 'ї')
}

fn is_iotated(c: char) -> bool {
    matches!(c, 'я' | 'ю' | 'є' | 'ї')
}

fn always_hard(c: char) -> bool {
    matches!(c, 'ж' | 'ш' | 'ц')
}

fn consonant(c: char) -> Option<Cons> {
    let ob = |hard, soft, voiced| Cons { hard, soft, glide: false, voiced, assimilate: true, spread: true, sonorant: false };
    let son = |hard, soft| Cons { hard, soft, glide: false, voiced: true, assimilate: false, spread: false, sonorant: true };
    let lab = |hard, soft, voiced, assimilate, spread| Cons {
        hard,
        soft,
        glide: true,
        voiced,
        assimilate,
        spread,
        sonorant: !assimilate && !spread,
    };
    Some(match c {
        'п' => lab("p", "pʲ", false, true, true),
        'б' => lab("b", "bʲ", true, true, true),
        'ф' => lab("f", "fʲ", false, true, true),
        'в' => lab("v", "vʲ", true, true, false),
        'м' => lab("m", "mʲ", true, false, false),
        'р' => lab("r", "rʲ", true, false, false),
        'т' => ob("t", "tʲ", false),
        'д' => ob("d", "dʲ", true),
        'с' => ob("s", "sʲ", false),
        'з' => ob("z", "zʲ", true),
        'н' => son("n", "nʲ"),
        'л' => son("l", "lʲ"),
        'к' => ob("k", "kʲ", false),
        'ґ' => ob("g", "gʲ", true),
        'г' => Cons { hard: "ɦ", soft: "ɦ", glide: false, voiced: true, assimilate: true, spread: true, sonorant: false },
        'х' => Cons { hard: "x", soft: "xʲ", glide: false, voiced: false, assimilate: false, spread: true, sonorant: false },
        'ш' => Cons { hard: "ʃ", soft: "ʃ", glide: false, voiced: false, assimilate: true, spread: true, sonorant: false },
        'ж' => Cons { hard: "ʒ", soft: "ʒ", glide: false, voiced: true, assimilate: true, spread: true, sonorant: false },
        'ц' => Cons { hard: "ts", soft: "ts", glide: false, voiced: false, assimilate: false, spread: true, sonorant: false },
        'ч' => Cons { hard: "tʃ", soft: "tʃ", glide: false, voiced: false, assimilate: false, spread: true, sonorant: false },
        'й' => son("j", "j"),
        _ => return None,
    })
}

fn dict_stress(word: &str) -> Option<usize> {
    STRESS.binary_search_by_key(&word, |e| e.0).ok().map(|i| STRESS[i].1 as usize)
}

/// Vowel index from the start. Only words whose stress is not the last vowel, plus the
/// anthem line. Sorted for the binary search (UTF-8: `і` and `ґ` sort after `я`).
#[rustfmt::skip]
const STRESS: &[(&str, u8)] = &[
    ("вмерла", 0),
    ("вмерли", 0),
    ("волю", 0),
    ("воля", 0),
    ("волі", 0),
    ("голос", 0),
    ("дитина", 1),
    ("дякую", 0),
    ("дівчина", 0),
    ("мясо", 0),
    ("слава", 0),
    ("славу", 0),
    ("україна", 2),
    ("україни", 2),
    ("україною", 2),
    ("україну", 2),
    ("їжак", 1),
    ("ґанок", 0),
];

#[cfg(test)]
mod lexicon_order {
    #[test]
    fn the_stress_list_is_sorted_for_the_binary_search() {
        for pair in super::STRESS.windows(2) {
            assert!(pair[0].0 < pair[1].0, "{} >= {}", pair[0].0, pair[1].0);
        }
    }
}
