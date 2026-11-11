//! Russian lyric front end: Cyrillic to the inventory, through the same syllable path as English.
use chimera::airway::frequency::{find_peaks, transfer_function};
use chimera::anatomy::human::{ArticKey, BODY_MALE, BodyMix, HumanArtic, NEUTRAL};
use chimera::voice::human_airway::HumanAirway;
use chimera::phonetics::inventory::phoneme;
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
    assert_eq!(ipa("мы", false), "m ᵻ");
    assert_eq!(ipa("ты", false), "t ᵻ");
    assert_eq!(ipa("ти", false), "tʲ i");
    // х is the velar fricative; ль is a palatalized lateral, and a final voiced stop devoices
    assert_eq!(ipa("хлеб", false), "x lʲ ɛ p");
    // ш is retroflex; щ is the long alveolo-palatal, and ч is an affricate into that fricative
    assert_eq!(ipa("шум", false), "ʂ u m");
    assert_eq!(ipa("щи", false), "ɕ i");
    assert_eq!(ipa("час", false), "tɕ ä s");
    let soft = phoneme("pʲ", BodyMix::MALE).expect("pʲ");
    assert!(soft.dominant.contains(&ArticKey::TbY), "the palatalization is its own gesture");
    assert!(soft.targets.get(ArticKey::LipAp).is_some());
    assert!(soft.targets.get(ArticKey::TbY).unwrap() > 7.5);
    let shch = phoneme("ɕ", BodyMix::MALE).unwrap();
    let sh = phoneme("ʂ", BodyMix::MALE).unwrap();
    assert!(shch.dur > sh.dur, "щ is the long fricative");
    assert_eq!(phoneme("tɕ", BodyMix::MALE).unwrap().release, Some("ɕ"));
}

#[test]
fn stress_marks_choose_reduction_and_clitics_reduce_without_one() {
    assert_eq!(ipa("ма́ма", false), "m ä m ɐ");
    assert_eq!(ipa("мама́", false), "m ɐ m ä");
    // singing keeps an open central vowel; speech uses schwa off the pretonic syllable
    assert_eq!(ipa("молоко́", false), "m ɐ l ɐ k o");
    assert_eq!(ipa("молоко́", true), "m ə l ɐ k o");
    // no mark: the last vowel, unless the word is in the authored list (мама is not мама́)
    assert_eq!(ipa("молоко", false), "m ɐ l ɐ k o");
    assert_eq!(ipa("мама", false), "m ä m ɐ");
    assert_eq!(ipa("поле", false), ipa("по́ле", false));
    // a mark wins over the list
    assert_eq!(ipa("мама́", false), "m ɐ m ä");
    // an unknown word still reduces everything but one vowel
    assert_eq!(ipa("бароба", false), "b ɐ r ɐ b ä");
    // unstressed у is the lax vowel; ы is the backer Russian vowel, not shared ɨ
    assert_eq!(ipa("улыбка", false), "ʊ l ᵻ p k ɐ");
    // stem stress, so the stressed а/о stay full and the others reduce
    assert_eq!(ipa("роботы", false), "r o b ɐ t ᵻ");
    assert_eq!(ipa("вкалывают", false), "f k ä l ᵻ v ɐ j ʊ t");
    assert_eq!(ipa("человек", false), "tɕ ɪ l ɐ vʲ ɛ k");
    // the suffix-stressed lookalike is in the list, so the -ыва- rule does not steal it
    assert_eq!(ipa("забывают", false), ipa("забыва́ют", false));
    // a word the list does not have still takes the vowel before -ыва-
    assert_eq!(ipa("рассказывают", false), ipa("расска́зывают", false));
    // vowel-initial ё/ю/я/е keep the glide
    assert_eq!(ipa("еда", false), "j ɪ d ä");
    assert_eq!(ipa("юла", false), "j ʊ l ä");
    assert_eq!(ipa("яма", false), "j ä m ɐ");
    assert_eq!(ipa("моя", false), "m ɐ j ä");
    // ё is stressed; a bare monosyllable is stressed; во is a clitic unless marked
    assert_eq!(ipa("ёлка", false), "j o l k ɐ");
    assert_eq!(ipa("во", false), "v ɐ");
    assert_eq!(ipa("во́", false), "v o");
    // apostrophe marks stress the same way an acute does
    assert_eq!(ipa("мо'ре", false), ipa("мо́ре", false));
}

#[test]
fn palatalization_voicing_and_the_soft_sign() {
    assert_eq!(ipa("день", false), "dʲ ɛ nʲ");
    assert_eq!(ipa("дуб", false), "d u p");
    assert_eq!(ipa("вокза́л", false), "v ɐ g z ä l");
    assert_eq!(ipa("всё", false), "f sʲ o");
    assert_eq!(ipa("семья́", false), "sʲ ɪ mʲ j ä");
    // softness spreads through a coronal cluster, then the cluster voices, and the gesture stays
    assert_eq!(ipa("сде́лать", false), "zʲ dʲ ɛ l ɐ tʲ");
    // before и the softness is on the consonant, with no extra j; before я the j remains
    assert_eq!(ipa("пи", false), "pʲ i");
    assert_eq!(ipa("пя́ть", false), "pʲ j ä tʲ");
    // и after a husher is ы
    assert_eq!(ipa("жир", false), "ʐ ᵻ r");
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
        ["bʲ ɪ", "rʲ j o", "z ɐ"]
    );
    assert!(syl[2].word_end && !syl[0].word_end);
    // the hyphen is not a word boundary: з stays voiced, and за is reduced from the ё
    let whole = sung("берёза");
    assert_eq!(
        whole.iter().map(phones).collect::<Vec<_>>(),
        ["bʲ ɪ", "rʲ j o", "z ɐ"]
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
    assert_eq!(phones(&held[3]), "l ä");
    // a lone за is the preposition, and reduces
    assert_eq!(ipa("за", false), "z ɐ");
    // an unstressed и that opens a hyphen piece with a later vowel is the glide of that note
    let spring = sung("ивес-на́");
    assert_eq!(spring.len(), 2);
    assert_eq!(
        spring.iter().map(phones).collect::<Vec<_>>(),
        ["j vʲ ɪ s", "n ä"]
    );
    let tied = sung("и_вес-на́");
    assert_eq!(tied.len(), 2);
    assert_eq!(phones(&tied[0]), "j vʲ ɪ s");
    // the same vowel hyphenated on its own note stays a nucleus, and so does a reduced у
    assert_eq!(sung("и-вес-на́").len(), 3);
    let smile = sung("у-лы́б-ку");
    assert_eq!(smile.len(), 3);
    assert_eq!(
        smile.iter().map(phones).collect::<Vec<_>>(),
        ["ʊ", "l ᵻ p", "k ʊ"]
    );
    // the same rule turns a leading unstressed у into w when it shares the note
    let lessons = sung("уро-ки");
    assert_eq!(lessons.len(), 2);
    assert_eq!(
        lessons.iter().map(phones).collect::<Vec<_>>(),
        ["w r ɐ", "kʲ i"]
    );
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
        ["v ɐ g", "z ä l"]
    );
    let st = sung("стоя́ла");
    assert_eq!(
        st.iter().map(phones).collect::<Vec<_>>(),
        ["s t ɐ", "j ä", "l ɐ"]
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
        ["ä", "ɐ"]
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
            "ɐ", "o", "ɪ", "ɪ", "o", "ɐ", "ɐ", "ä", "ɐ", "ɐ", "o", "ɪ", "ʊ", "ä", "ɐ", "ɐ", "ɐ",
            "ä", "ɐ"
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

fn peaks_of(ipa: &str) -> Vec<(f64, f64)> {
    use chimera::phonetics::planner::vowel_artic;
    let patch = vowel_artic(ipa, BodyMix::MALE).unwrap();
    let a: HumanArtic = patch.apply_to(&NEUTRAL);
    let mut w = HumanAirway::new(96_000.0, BodyMix::MALE.body());
    w.apply(&a, 0.0);
    let el = w.net.linear_elements();
    let freqs: Vec<f64> = (0..360).map(|i| 60.0 * (8000.0f64 / 60.0).powf(i as f64 / 359.0)).collect();
    let h = transfer_function(&el, w.pharynx.first_node(), &freqs, &[w.glottis_port]);
    find_peaks(&freqs, &h.mag, 150.0)
        .into_iter()
        .take(4)
        .map(|p| (p.freq, p.db))
        .collect()
}

#[test]
fn russian_y_and_stressed_a_clear_the_male_formants() {
    let band = |ipa: &str, lo: f64, hi: f64| {
        peaks_of(ipa)
            .into_iter()
            .filter(|(f, _)| *f >= lo && *f < hi)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap_or((0.0, -99.0))
    };
    let (y2, y2db) = band("ᵻ", 900.0, 1800.0);
    let (y3, y3db) = band("ᵻ", 2000.0, 2600.0);
    let (i2, _) = band("i", 1800.0, 2600.0);
    let (a2, _) = band("ä", 1400.0, 2000.0);
    let (schwa, _) = band("ɐ", 1100.0, 1700.0);
    assert!(i2 - y2 > 600.0, "ы F2 {y2} is too close to и F2 {i2}");
    assert!(y2db > y3db, "ы F2 {y2}@{y2db} should outrank the band near и ({y3}@{y3db})");
    assert!(a2 - schwa > 150.0, "stressed а F2 {a2} does not clear ɐ F2 {schwa}");
}
