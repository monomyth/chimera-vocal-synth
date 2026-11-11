//! Russian lyric front end: Cyrillic to the inventory, through the same syllable path as English.
use chimera::anatomy::human::BODY_MALE;
use chimera::math::midi_to_hz;
use chimera::phonetics::Syllable;
use chimera::phonetics::english::EnglishG2P;
use chimera::phonetics::russian::RussianG2P;
use chimera::phonetics::sing::{SongOptions, SungNote, plan_song};
use chimera::phonetics::syllables::{LyricLang, lyric_language, lyrics_to_syllables, sing_lyrics};
use chimera::voice::human_voice::{HumanVoice, SourcePatch};

fn ipa(w: &str, speech: bool) -> String {
    RussianG2P
        .word(w, speech)
        .iter()
        .map(|s| s.ipa.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}
fn sung(text: &str) -> Vec<Syllable> {
    lyrics_to_syllables(text, &RussianG2P)
}
fn phones(s: &Syllable) -> String {
    s.onset
        .iter()
        .chain(std::iter::once(&s.nucleus))
        .chain(s.coda.iter())
        .map(|p| p.ipa.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn inventory_gaps_map_onto_phonemes_the_tract_has() {
    // ы is the calibrated central vowel, not и or у
    assert_eq!(ipa("мы", false), "m ɨ");
    assert_eq!(ipa("ты", false), "t ɨ");
    assert_eq!(ipa("ти", false), "c i");
    // х is the velar fricative; a final voiced stop devoices
    assert_eq!(ipa("хлеб", false), "x ʎ ɛ p");
    // ш is retroflex; щ has no ɕ, so it is the palatal fricative, and ч stays an affricate
    assert_eq!(ipa("шум", false), "ʂ u m");
    assert_eq!(ipa("щи", false), "ç i");
    assert_eq!(ipa("час", false), "tʃ a s");
}

#[test]
fn stress_marks_choose_reduction_and_clitics_reduce_without_one() {
    assert_eq!(ipa("ма́ма", false), "m a m ɐ");
    assert_eq!(ipa("мама́", false), "m ɐ m a");
    // singing keeps an open central vowel; speech uses schwa off the pretonic syllable
    assert_eq!(ipa("молоко́", false), "m ɐ l ɐ k o");
    assert_eq!(ipa("молоко́", true), "m ə l ɐ k o");
    // no mark on a longer word: full vowels, so the missing accent is audible
    assert_eq!(ipa("молоко", false), "m o l o k o");
    // ё is stressed; a bare monosyllable is stressed; во is a clitic unless marked
    assert_eq!(ipa("ёлка", false), "j o l k ɐ");
    assert_eq!(ipa("во", false), "v ɐ");
    assert_eq!(ipa("во́", false), "v o");
    // apostrophe marks stress the same way an acute does
    assert_eq!(ipa("мо'ре", false), ipa("мо́ре", false));
}

#[test]
fn palatalization_voicing_and_the_soft_sign() {
    assert_eq!(ipa("день", false), "ɟ ɛ ɲ");
    assert_eq!(ipa("дуб", false), "d u p");
    assert_eq!(ipa("вокза́л", false), "v ɐ g z a l");
    assert_eq!(ipa("всё", false), "f ç o");
    assert_eq!(ipa("семья́", false), "ç ɪ m j a");
    // softness spreads through a coronal cluster, then the cluster voices
    assert_eq!(ipa("сде́лать", false), "ʝ ɟ ɛ l ɐ c");
    // и after a husher is ы
    assert_eq!(ipa("жир", false), "ʐ ɨ r");
}

#[test]
fn hyphens_are_sung_breaks_and_melisma_still_holds_the_vowel() {
    let syl = sung("бе-рё-за");
    assert_eq!(
        syl.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
        ["бе", "рё", "за"]
    );
    assert_eq!(
        syl.iter().map(phones).collect::<Vec<_>>(),
        ["b ɪ", "r j o", "z ɐ"]
    );
    assert!(syl[2].word_end && !syl[0].word_end);
    // the hyphen is not a word boundary: з stays voiced, and за is reduced from the ё
    let whole = sung("берёза");
    assert_eq!(
        whole.iter().map(phones).collect::<Vec<_>>(),
        ["b ɪ", "r j o", "z ɐ"]
    );
    assert_eq!(
        whole.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
        ["бе", "рё", "за"]
    );

    let held = sung("бе-рё - ла");
    assert_eq!(
        held.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
        ["бе", "рё", "~", "ла"]
    );
    assert_eq!(held[2].melisma, Some(true));
    assert_eq!(held[2].nucleus.ipa, "o");
    // the spaced hyphen ended the word, so ла is its own stressed syllable
    assert_eq!(phones(&held[3]), "l a");
    // a lone за is the preposition, and reduces
    assert_eq!(ipa("за", false), "z ɐ");
}

#[test]
fn unhyphenated_words_split_on_russian_onsets() {
    let vz = sung("вокза́л");
    assert_eq!(
        vz.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
        ["вок", "за́л"]
    );
    assert_eq!(
        vz.iter().map(phones).collect::<Vec<_>>(),
        ["v ɐ g", "z a l"]
    );
    let st = sung("стоя́ла");
    assert_eq!(
        st.iter().map(phones).collect::<Vec<_>>(),
        ["s t ɐ", "j a", "l ɐ"]
    );
}

#[test]
fn language_field_selects_the_front_end_and_english_text_stays_english() {
    assert_eq!(lyric_language("hello", None).unwrap(), LyricLang::En);
    assert_eq!(lyric_language("Во по́ле", None).unwrap(), LyricLang::Ru);
    assert_eq!(
        lyric_language("Во по́ле", Some("en")).unwrap(),
        LyricLang::En
    );
    assert_eq!(lyric_language("hello", Some("ru")).unwrap(), LyricLang::Ru);
    assert!(lyric_language("Во", Some("de")).is_err());
    // an explicit English reading still drops Cyrillic (no English syllable), as the old tokenizer did
    let en = sing_lyrics("hello Во", Some("en"), &EnglishG2P::new()).unwrap();
    assert_eq!(
        en.len(),
        sing_lyrics("hello", None, &EnglishG2P::new())
            .unwrap()
            .len()
    );
    let ru = sing_lyrics("ма́ма", None, &EnglishG2P::new()).unwrap();
    assert_eq!(
        ru.iter()
            .map(|s| s.nucleus.ipa.as_str())
            .collect::<Vec<_>>(),
        ["a", "ɐ"]
    );
}

#[test]
fn the_folk_line_has_one_syllable_per_note() {
    let path = format!(
        "{}/tests/fixtures/vo-pole-bereza.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let j: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let lyrics = j["lyrics"].as_str().unwrap();
    let syl = sing_lyrics(lyrics, j["language"].as_str(), &EnglishG2P::new()).unwrap();
    let notes = j["notes"].as_array().unwrap();
    let sounding = notes
        .iter()
        .filter(|n| n[0].as_f64().unwrap() > 0.0)
        .count();
    assert_eq!(
        syl.len(),
        sounding,
        "{}",
        syl.iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );
    let nuclei: Vec<&str> = syl.iter().map(|s| s.nucleus.ipa.as_str()).collect();
    assert_eq!(
        nuclei,
        [
            "ɐ", "o", "ɪ", "ɪ", "o", "ɐ", "ɐ", "a", "ɐ", "ɐ", "o", "ɪ", "u", "a", "ɐ", "ɐ", "ɐ",
            "a", "ɐ"
        ]
    );
    assert!(syl[8].phrase_end, "the comma breathes after the first line");
}

#[test]
fn two_notes_render_finite_voiced_audio() {
    let syl = sung("ма́-ма");
    assert_eq!(syl.len(), 2);
    let notes: Vec<SungNote> = syl
        .into_iter()
        .enumerate()
        .map(|(i, s)| SungNote {
            midi: if i == 0 { 60.0 } else { 62.0 },
            beats: 1.0,
            syl: s,
        })
        .collect();
    let plan = plan_song(
        &notes,
        &SongOptions {
            body: "male".into(),
            bpm: 100.0,
            t0: 0.2,
            vibrato: 0.0,
        },
    );
    let mut v = HumanVoice::new(48_000.0, BODY_MALE, 2);
    v.set_source(&SourcePatch {
        f0: Some(midi_to_hz(60.0)),
        voiced: Some(true),
        pressure: Some(9.0),
        tension: Some(0.55),
        ..Default::default()
    });
    v.schedule(&plan.gestures);
    let n = (48_000.0 * (plan.end + 0.25)) as usize;
    let mut out = vec![0.0f32; n];
    v.render(&mut out);
    assert!(
        out.iter().all(|s| s.is_finite()),
        "the airway went non-finite"
    );
    let peak = out.iter().copied().map(f32::abs).fold(0.0f32, f32::max);
    assert!(peak > 0.02, "peak {peak}: the syllables made no sound");
}
