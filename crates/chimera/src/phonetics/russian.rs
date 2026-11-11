//! Russian lyrics → inventory IPA.
//!
//! Moscow reading, sung rather than spoken: stress is an acute on the vowel (`мо́ре`), an
//! apostrophe after it (`мо'ре`), or `ё` (always stressed). A monosyllable with no mark is
//! stressed, except a small list of clitics (`во`, `на`, `не`, …) which stay reduced. A longer
//! word with no mark takes [`dict_stress`] when the word is in that authored list, then the
//! vowel before an `-ыва-`/`-ива-` suffix, and otherwise the last vowel, so the other vowels
//! still reduce. Unstressed `а`/`о` become `ɐ` when singing (a held note stays a vowel) and,
//! in speech (`reduce`), `ɐ` in the pretonic syllable and `ə` elsewhere. Unstressed `е`/`я`
//! after a consonant become `ɪ`. Unstressed `у`/`ю` become `ʊ`. Stressed `а`/`я` are `ä` and
//! `ы` is `ᵻ`: those two symbols are not the shared `a` and `ɨ`, whose formants sit on top of
//! `ɐ` and `i` at a male pitch.
//!
//! A soft consonant is the same constriction plus a palatal tongue-body gesture (`п` → `pʲ`,
//! `т` → `tʲ`, `л` → `lʲ`, `х` before a soft vowel → `xʲ`). Soft labials and `р` still add `j`
//! before a back vowel. `ы` is `ᵻ` (not the shared `ɨ`), hard `х` is `x`, `ш`/`ж` are retroflex `ʂ`/`ʐ`. `щ` is the
//! long alveolo-palatal `ɕ` and `ч` is the affricate `tɕ`, so they differ from `ш`/`ч` of
//! English (`ʂ` / `tʃ`) in place and from each other in manner.
//!
//! Typed hyphens are the sung syllable breaks. They are not word boundaries: voicing and
//! reduction run across them (`бе-рё-за` keeps a voiced `з`). In a hyphenated word, an
//! unstressed `и` or `у` at the start of a piece that also has another vowel is a glide on that
//! note (`ивес-на́`, or `и_вес-на́` with a tie), not a second nucleus.

use super::Segment;
use super::syllables::{SyllableParts, WordSource, syllabify_with};

/// Word-initial clusters the maximal-onset split may give the next syllable (IPA, the way
/// [`super::syllables`] matches them). Single consonants are every segment this front end emits.
/// `rj` is an onset so unhyphenated `берёза` is бе-рё-за; `drj` is not, so `кудрявая` keeps the
/// `д` on `куд`.
#[rustfmt::skip]
const ONSETS: &[&str] = &[
    "p", "b", "t", "d", "k", "g", "m", "n", "f", "v", "s", "z", "ʂ", "ʐ", "x",
    "ts", "tʃ", "ç", "ʝ", "c", "ɟ", "ɲ", "ʎ", "r", "l", "j", "w",
    "pʲ", "bʲ", "tʲ", "dʲ", "kʲ", "gʲ", "mʲ", "nʲ", "fʲ", "vʲ", "sʲ", "zʲ", "xʲ", "lʲ", "rʲ", "ɕ", "tɕ",
    "pr", "br", "tr", "dr", "kr", "gr", "fr", "vr",
    "pl", "bl", "kl", "gl", "fl",
    "sp", "st", "sk", "sv", "zv", "dv", "tv",
    "pj", "bj", "mj", "fj", "vj", "rj",
    "pʲj", "bʲj", "mʲj", "fʲj", "vʲj", "rʲj",
    "str", "spr", "skr",
];

const CLITICS: &[&str] = &[
    "в",
    "во",
    "на",
    "не",
    "ни",
    "и",
    "а",
    "но",
    "к",
    "ко",
    "с",
    "со",
    "о",
    "об",
    "от",
    "до",
    "за",
    "из",
    "у",
    "по",
    "под",
    "над",
    "при",
    "без",
    "для",
    "чтоб",
    "чтобы",
    "же",
    "ли",
    "бы",
    "уж",
];

#[derive(Clone, Copy)]
struct Letter {
    ch: char,
    part: usize,
    marked: bool,
}

struct Unit {
    ipa: &'static str,
    part: usize,
    vowel: bool,
    stress: u8,
    /// skipped while looking for a voicing trigger (sonorants, and `в`)
    skip: bool,
    assimilate: bool,
    /// underlying voicing spreads left. `в` assimilates but does not spread voice.
    spread: bool,
    underlying_voiced: bool,
}

#[derive(Clone, Copy)]
struct Cons {
    hard: &'static str,
    soft: &'static str,
    /// soft labial or `р`: the hard consonant, plus `j` before a back vowel
    glide: bool,
    voiced: bool,
    assimilate: bool,
    spread: bool,
    sonorant: bool,
}

/// Russian grapheme-to-phoneme. `reduce` on [`WordSource::word`] is speech reduction (schwa);
/// singing passes `false` and still reduces, to `ɐ` rather than `ə`.
#[derive(Clone, Copy, Debug, Default)]
pub struct RussianG2P;

impl RussianG2P {
    pub fn word(&self, w: &str, speech: bool) -> Vec<Segment> {
        let flat = w.replace('-', "");
        flatten(&pronounce(&[flat.as_str()], speech))
    }
}

impl WordSource for RussianG2P {
    fn word(&self, w: &str, reduce: bool) -> Vec<Segment> {
        RussianG2P::word(self, w, reduce)
    }
    fn onsets(&self) -> &'static [&'static str] {
        ONSETS
    }
    fn word_hyphenated(&self, parts: &[&str], reduce: bool) -> Option<Vec<Vec<SyllableParts>>> {
        Some(pronounce(parts, reduce))
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

fn pronounce(parts: &[&str], speech: bool) -> Vec<Vec<SyllableParts>> {
    let letters = parse_parts(parts);
    let n = parts.len().max(1);
    if letters.is_empty() {
        return vec![vec![]; n];
    }
    let stressed = stress_marks(&letters, parts);
    let pretonic = pretonic_of(&letters, &stressed);
    let soft = softness(&letters);
    let mut units = Vec::new();
    for i in 0..letters.len() {
        let g = letters[i];
        if is_vowel_letter(g.ch) {
            if let Some(gipa) = glide_off(&letters, i, stressed[i], parts.len()) {
                units.push(glide_of(gipa, g.part));
                continue;
            }
            let ipa = vowel_ipa(&letters, &soft, i, stressed[i], pretonic[i], speech);
            if inserts_j(&letters, i) {
                units.push(glide(g.part));
            }
            units.push(Unit {
                ipa,
                part: g.part,
                vowel: true,
                stress: if stressed[i] { 1 } else { 0 },
                skip: false,
                assimilate: false,
                spread: false,
                underlying_voiced: true,
            });
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
        for c in part.to_lowercase().chars() {
            if c == '\u{0301}' || c == '\'' {
                if let Some(last) = out.last_mut()
                    && last.part == pi
                    && is_vowel_letter(last.ch)
                {
                    last.marked = true;
                }
                continue;
            }
            if c == '-' || (!is_vowel_letter(c) && consonant(c).is_none() && c != 'ь' && c != 'ъ')
            {
                continue;
            }
            out.push(Letter {
                ch: c,
                part: pi,
                marked: c == 'ё',
            });
        }
    }
    out
}

fn bare(parts: &[&str]) -> String {
    let mut s = String::new();
    for p in parts {
        for c in p.to_lowercase().chars() {
            if c != '\u{0301}' && c != '\'' && c != '-' && c != '_' && c != '\u{203F}' {
                s.push(c);
            }
        }
    }
    s
}

fn stress_marks(letters: &[Letter], parts: &[&str]) -> Vec<bool> {
    let vowels: Vec<usize> = (0..letters.len())
        .filter(|&i| is_vowel_letter(letters[i].ch))
        .collect();
    let marked = vowels.iter().any(|&i| letters[i].marked);
    let mut stressed = vec![false; letters.len()];
    if marked {
        for i in vowels {
            stressed[i] = letters[i].marked;
        }
    } else if CLITICS.contains(&bare(parts).as_str()) {
        // every vowel stays unstressed
    } else if let Some(v) = dict_stress(&bare(parts)) {
        if let Some(&i) = vowels.get(v) {
            stressed[i] = true;
        }
    } else if let Some(v) = yva_stress(&letters) {
        // productive -ыва-/-ива- puts stress on the vowel before the suffix. The list above
        // holds the lookalikes that stress the suffix instead (забыва́ют, открыва́ть).
        if let Some(&i) = vowels.get(v) {
            stressed[i] = true;
        }
    } else if vowels.len() <= 1 {
        if let Some(i) = vowels.first() {
            stressed[*i] = true;
        }
    } else if let Some(&i) = vowels.last() {
        // unknown word: one stress, on the last vowel, so the others still reduce
        stressed[i] = true;
    }
    stressed
}

/// The vowel immediately before a stressed one (Moscow's first pretonic).
fn pretonic_of(letters: &[Letter], stressed: &[bool]) -> Vec<bool> {
    let mut pre = vec![false; letters.len()];
    let mut next_stressed = false;
    for i in (0..letters.len()).rev() {
        if !is_vowel_letter(letters[i].ch) {
            continue;
        }
        pre[i] = !stressed[i] && next_stressed;
        next_stressed = stressed[i];
    }
    pre
}

fn softness(letters: &[Letter]) -> Vec<bool> {
    let mut soft = vec![false; letters.len()];
    for i in (0..letters.len()).rev() {
        let c = letters[i].ch;
        if consonant(c).is_none() {
            continue;
        }
        soft[i] = if always_hard(c) {
            false
        } else if always_soft(c) {
            true
        } else {
            match letters.get(i + 1).map(|g| g.ch) {
                Some('ь') => true,
                Some('ъ') => false,
                Some(n) if is_soft_vowel(n) => true,
                Some(n) if is_coronal(c) && is_coronal(n) && soft[i + 1] => true,
                _ => false,
            }
        };
    }
    soft
}

fn emit_cons(units: &mut Vec<Unit>, letters: &[Letter], i: usize, c: Cons, soft: bool) {
    let part = letters[i].part;
    let front = next_vowel_ipa(letters, i).is_some_and(is_front);
    let vowel_j = next_vowel_index(letters, i).is_some_and(|v| inserts_j(letters, v));
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

fn glide(part: usize) -> Unit {
    glide_of("j", part)
}

fn glide_of(ipa: &'static str, part: usize) -> Unit {
    Unit {
        ipa,
        part,
        vowel: false,
        stress: 0,
        skip: true,
        assimilate: false,
        spread: false,
        underlying_voiced: true,
    }
}

/// Regressive voicing, then word-final devoicing. `в` takes the voicing to its right and does
/// not pass its own voice left (`Москва` keeps `к`). Sonorants are transparent, so a final
/// obstruent still devoices in `зубр`.
fn assimilate(units: &mut [Unit]) {
    let mut force: Option<bool> = Some(false);
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
        ("ʂ", true) => "ʐ",
        ("ʐ", false) => "ʂ",
        ("k", true) => "g",
        ("g", false) => "k",
        ("c", true) => "ɟ",
        ("ɟ", false) => "c",
        ("ç", true) => "ʝ",
        ("ʝ", false) => "ç",
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
        let seg = Segment {
            ipa: u.ipa.to_string(),
            stress: Some(u.stress),
            ..Default::default()
        };
        if let Some(b) = buckets.get_mut(u.part) {
            b.push(seg);
        }
    }
    buckets
        .into_iter()
        .map(|segs| {
            let vowels: Vec<usize> = segs
                .iter()
                .enumerate()
                .filter(|(_, s)| is_vowel_ipa(&s.ipa))
                .map(|(i, _)| i)
                .collect();
            if vowels.is_empty() {
                // a consonant-only piece still has to occupy its note
                return vec![SyllableParts {
                    onset: segs,
                    nucleus: Segment::new("ə"),
                    coda: vec![],
                }];
            }
            if vowels.len() == 1 {
                let v = vowels[0];
                return vec![SyllableParts {
                    onset: segs[..v].to_vec(),
                    nucleus: segs[v].clone(),
                    coda: segs[v + 1..].to_vec(),
                }];
            }
            syllabify_with(&segs, ONSETS)
        })
        .collect()
}

fn is_vowel_ipa(ipa: &str) -> bool {
    matches!(ipa, "a" | "ä" | "ɛ" | "o" | "u" | "ʊ" | "i" | "ɨ" | "ᵻ" | "ɪ" | "ɐ" | "ə")
}

/// In a hyphenated word, an unstressed `и` or `у` that opens a piece and is followed by another
/// vowel of that same piece is the glide of that note (`ивес-на́` → `j` + `вес`).
fn glide_off(letters: &[Letter], i: usize, stressed: bool, parts: usize) -> Option<&'static str> {
    if parts < 2 || stressed {
        return None;
    }
    let ch = letters[i].ch;
    if ch != 'и' && ch != 'у' {
        return None;
    }
    let part = letters[i].part;
    let earlier = (0..i).any(|k| letters[k].part == part && is_vowel_letter(letters[k].ch));
    let later = ((i + 1)..letters.len()).any(|k| letters[k].part == part && is_vowel_letter(letters[k].ch));
    if earlier || !later {
        return None;
    }
    Some(if ch == 'и' { "j" } else { "w" })
}

fn next_vowel_index(letters: &[Letter], i: usize) -> Option<usize> {
    ((i + 1)..letters.len()).find(|&k| is_vowel_letter(letters[k].ch))
}

fn next_vowel_ipa(letters: &[Letter], i: usize) -> Option<&'static str> {
    // quality is decided from the letters, so this does not need the stress tables: only
    // frontness matters here, and every front target (`i ɪ ɛ`) stays front whether or not the
    // vowel is the one reduction would pick. Read the letter.
    let k = next_vowel_index(letters, i)?;
    let ch = letters[k].ch;
    if is_front_letter(ch) || (inserts_j(letters, k) && matches!(ch, 'е' | 'я' | 'ё' | 'и')) {
        // `я`/`ё` after a consonant are back vowels with a soft onset; word-initial `е` is front.
        if matches!(ch, 'я' | 'ё') && !inserts_j(letters, k) {
            return Some("a");
        }
        if ch == 'ё' {
            return Some("o");
        }
        Some("i")
    } else {
        Some("a")
    }
}

fn is_front(ipa: &str) -> bool {
    matches!(ipa, "i" | "ɪ" | "e" | "ɛ")
}

fn is_front_letter(c: char) -> bool {
    matches!(c, 'и' | 'е' | 'э')
}

fn vowel_ipa(
    letters: &[Letter],
    soft: &[bool],
    i: usize,
    stressed: bool,
    pretonic: bool,
    speech: bool,
) -> &'static str {
    let ch = letters[i].ch;
    let husher = after_husher(letters, i);
    let j = inserts_j(letters, i);
    let prev_vowel = prev_letter(letters, i).is_some_and(is_vowel_letter);
    // `а` after a soft consonant reduces like `я` (`часы` → ɪ), not like hard `а`.
    let soft_a = ch == 'а' && consonant_before(letters, i).is_some_and(|k| soft[k]);
    if husher && (matches!(ch, 'и' | 'ы') || (!stressed && matches!(ch, 'е' | 'э'))) {
        return "ᵻ";
    }
    if stressed {
        return match ch {
            'а' | 'я' => "ä",
            'э' | 'е' => "ɛ",
            'о' | 'ё' => "o",
            'у' | 'ю' => "u",
            'ы' => "ᵻ",
            _ => "i",
        };
    }
    match ch {
        'у' | 'ю' => "ʊ",
        'ы' | 'э' => "ᵻ",
        'и' => "i",
        'а' if soft_a => "ɪ",
        'а' | 'о' => open_reduced(speech, pretonic),
        'е' | 'я' | 'ё' if j && prev_vowel => open_reduced(speech, pretonic),
        'е' | 'я' | 'ё' => "ɪ",
        _ => "ə",
    }
}

fn consonant_before(letters: &[Letter], i: usize) -> Option<usize> {
    let mut k = i;
    while k > 0 {
        k -= 1;
        if consonant(letters[k].ch).is_some() {
            return Some(k);
        }
        if is_vowel_letter(letters[k].ch) {
            return None;
        }
    }
    None
}

/// Singing holds an open central vowel. Speech uses that only in the pretonic syllable.
fn open_reduced(speech: bool, pretonic: bool) -> &'static str {
    if speech && !pretonic { "ə" } else { "ɐ" }
}

fn inserts_j(letters: &[Letter], i: usize) -> bool {
    let c = letters[i].ch;
    let iotated = matches!(c, 'я' | 'е' | 'ё' | 'ю');
    match prev_letter(letters, i) {
        None => iotated,
        Some(p) if is_vowel_letter(p) => iotated,
        Some('ь') => iotated || c == 'и',
        Some('ъ') => iotated,
        _ => false,
    }
}

fn after_husher(letters: &[Letter], i: usize) -> bool {
    match prev_letter(letters, i) {
        Some(c) if always_hard(c) => true,
        _ => false,
    }
}

fn prev_letter(letters: &[Letter], i: usize) -> Option<char> {
    i.checked_sub(1).map(|k| letters[k].ch)
}

fn is_vowel_letter(c: char) -> bool {
    matches!(c, 'а' | 'е' | 'ё' | 'и' | 'о' | 'у' | 'ы' | 'э' | 'ю' | 'я')
}
fn is_soft_vowel(c: char) -> bool {
    matches!(c, 'я' | 'е' | 'ё' | 'ю' | 'и')
}
fn always_hard(c: char) -> bool {
    matches!(c, 'ж' | 'ш' | 'ц')
}
fn always_soft(c: char) -> bool {
    matches!(c, 'ч' | 'щ' | 'й')
}
fn is_coronal(c: char) -> bool {
    matches!(c, 'т' | 'д' | 'с' | 'з')
}

fn consonant(c: char) -> Option<Cons> {
    let ob = |hard, soft, voiced| Cons {
        hard,
        soft,
        glide: false,
        voiced,
        assimilate: true,
        spread: true,
        sonorant: false,
    };
    let son = |hard, soft| Cons {
        hard,
        soft,
        glide: false,
        voiced: true,
        assimilate: false,
        spread: false,
        sonorant: true,
    };
    let glide = |hard, soft, voiced, assimilate, spread| Cons {
        hard,
        soft,
        glide: true,
        voiced,
        assimilate,
        spread,
        sonorant: !assimilate && !spread,
    };
    Some(match c {
        'п' => glide("p", "pʲ", false, true, true),
        'б' => glide("b", "bʲ", true, true, true),
        'ф' => glide("f", "fʲ", false, true, true),
        'в' => glide("v", "vʲ", true, true, false),
        'м' => glide("m", "mʲ", true, false, false),
        'р' => glide("r", "rʲ", true, false, false),
        'т' => ob("t", "tʲ", false),
        'д' => ob("d", "dʲ", true),
        'с' => ob("s", "sʲ", false),
        'з' => ob("z", "zʲ", true),
        'н' => son("n", "nʲ"),
        'л' => son("l", "lʲ"),
        'к' => ob("k", "kʲ", false),
        'г' => ob("g", "gʲ", true),
        'х' => Cons {
            hard: "x",
            soft: "xʲ",
            glide: false,
            voiced: false,
            assimilate: false,
            spread: true,
            sonorant: false,
        },
        'ш' => Cons {
            hard: "ʂ",
            soft: "ʂ",
            glide: false,
            voiced: false,
            assimilate: true,
            spread: true,
            sonorant: false,
        },
        'ж' => Cons {
            hard: "ʐ",
            soft: "ʐ",
            glide: false,
            voiced: true,
            assimilate: true,
            spread: true,
            sonorant: false,
        },
        'ц' => Cons {
            hard: "ts",
            soft: "ts",
            glide: false,
            voiced: false,
            assimilate: false,
            spread: true,
            sonorant: false,
        },
        'ч' => Cons {
            hard: "tɕ",
            soft: "tɕ",
            glide: false,
            voiced: false,
            assimilate: false,
            spread: true,
            sonorant: false,
        },
        'щ' => Cons {
            hard: "ɕ",
            soft: "ɕ",
            glide: false,
            voiced: false,
            assimilate: false,
            spread: true,
            sonorant: false,
        },
        'й' => son("j", "j"),
        _ => return None,
    })
}

/// Authored stem-stresses for common words whose stress is not the last vowel, and for the
/// `-ыва-` verbs that stress the suffix (the rule would put it before `-ыва-`). Not a copied
/// dictionary: homographs (`замок`, `мука`) are left out. The index is which vowel, from the start.
#[rustfmt::skip]
const STRESS: &[(&str, u8)] = &[
    ("белый", 0),
    ("береза", 1),
    ("видеть", 0),
    ("вкалывает", 0),
    ("вкалывал", 0),
    ("вкалывала", 0),
    ("вкалывали", 0),
    ("вкалывать", 0),
    ("вкалывают", 0),
    ("время", 0),
    ("город", 0),
    ("девочка", 0),
    ("девушка", 0),
    ("делать", 0),
    ("деньги", 0),
    ("дерево", 0),
    ("добывать", 2),
    ("добывают", 2),
    ("дорога", 1),
    ("думать", 0),
    ("забывал", 2),
    ("забывала", 2),
    ("забывали", 2),
    ("забывать", 2),
    ("забываю", 2),
    ("забывают", 2),
    ("завтра", 0),
    ("закрывать", 2),
    ("закрывают", 2),
    ("здравствуйте", 0),
    ("комната", 0),
    ("красивый", 1),
    ("красный", 0),
    ("кудрявая", 1),
    ("люди", 0),
    ("маленький", 0),
    ("мальчик", 0),
    ("мама", 0),
    ("море", 0),
    ("музыка", 0),
    ("небо", 0),
    ("новый", 0),
    ("открывал", 2),
    ("открывать", 2),
    ("открываю", 2),
    ("открывают", 2),
    ("папа", 0),
    ("песня", 0),
    ("пожалуйста", 1),
    ("покрывать", 2),
    ("покрывают", 2),
    ("поле", 0),
    ("работа", 1),
    ("рама", 0),
    ("раму", 0),
    ("робот", 0),
    ("робота", 0),
    ("роботам", 0),
    ("роботами", 0),
    ("роботов", 0),
    ("роботом", 0),
    ("роботу", 0),
    ("роботы", 0),
    ("родина", 0),
    ("русский", 0),
    ("свидания", 1),
    ("свобода", 1),
    ("сегодня", 1),
    ("сердце", 0),
    ("синий", 0),
    ("слово", 0),
    ("слышать", 0),
    ("собака", 1),
    ("солнце", 0),
    ("спасибо", 1),
    ("старый", 0),
    ("товарищ", 1),
    ("улица", 0),
    ("улыбка", 1),
    ("утро", 0),
    ("хороший", 1),
    ("черный", 0),
    ("чёрный", 0),
    ("школа", 0),
    ("яблоко", 0),
    ("яма", 0),
];

/// Vowel index of the vowel before an `-ыва-` / `-ива-` suffix, if the word has one.
fn yva_stress(letters: &[Letter]) -> Option<usize> {
    let vowels: Vec<usize> = (0..letters.len()).filter(|&i| is_vowel_letter(letters[i].ch)).collect();
    let i = (0..letters.len().saturating_sub(2)).find(|&i| {
        let c = letters[i].ch;
        (c == 'ы' || c == 'и') && letters[i + 1].ch == 'в' && letters[i + 2].ch == 'а'
    })?;
    let prev = vowels.iter().rev().find(|&&v| v < i).copied()?;
    vowels.iter().position(|&v| v == prev)
}

fn dict_stress(word: &str) -> Option<usize> {
    STRESS.binary_search_by_key(&word, |e| e.0).ok().map(|i| STRESS[i].1 as usize)
}

#[cfg(test)]
mod lexicon_order {
    #[test]
    fn the_stress_list_is_sorted_for_the_binary_search() {
        for pair in super::STRESS.windows(2) {
            assert!(pair[0].0 < pair[1].0, "{} >= {}", pair[0].0, pair[1].0);
        }
    }
}
