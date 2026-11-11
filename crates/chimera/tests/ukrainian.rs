//! Ukrainian through the same lyric path as Russian: full vowels, і/ї/є/ґ, г as ɦ.
use chimera::phonetics::english::EnglishG2P;
use chimera::phonetics::syllables::{LyricLang, lyric_language, sing_lyrics};
use chimera::phonetics::ukrainian::UkrainianG2P;

fn ipa(w: &str) -> String {
    UkrainianG2P.word(w, false).iter().map(|s| s.ipa.as_str()).collect::<Vec<_>>().join(" ")
}

fn stress_of(w: &str) -> Vec<u8> {
    UkrainianG2P.word(w, false).iter().map(|s| s.stress.unwrap_or(0)).collect()
}

#[test]
fn vowels_stay_full_and_the_ukrainian_letters_have_their_own_phones() {
    assert_eq!(ipa("не"), "n ɛ");
    assert_eq!(ipa("вода"), "v ɔ d a");
    assert_eq!(ipa("ми"), "m ɪ");
    assert_eq!(ipa("ті"), "tʲ i");
    assert_eq!(ipa("голос"), "ɦ ɔ l ɔ s");
    assert_eq!(ipa("ґанок"), "g a n ɔ k");
    assert_eq!(ipa("їжак"), "j i ʒ a k");
    assert_eq!(ipa("є"), "j ɛ");
    assert_eq!(ipa("м'ясо"), "m j a s ɔ");
    assert_eq!(ipa("з'їзд"), "z j i z d");
    assert_eq!(ipa("день"), "d ɛ nʲ");
    assert_eq!(ipa("льон"), "lʲ ɔ n");
    assert_eq!(ipa("ще"), "ʃ tʃ ɛ");
    assert_eq!(ipa("народ"), "n a r ɔ d");
    assert_eq!(ipa("віз"), "vʲ i z");
    assert_eq!(stress_of("не"), vec![0, 0]);
    assert_eq!(stress_of("вмерла"), vec![0, 0, 1, 0, 0, 0]);
    let voda = stress_of("вода");
    assert_eq!(voda[1], 0, "unstressed о stays ɔ, stress {voda:?}");
    assert_eq!(voda[3], 1);
}

#[test]
fn the_anthem_line_has_one_syllable_per_word_piece() {
    let line = "Ще не вмерла України і слава, і воля";
    let syl = sing_lyrics(line, Some("uk"), &EnglishG2P::new()).unwrap();
    let nuclei: Vec<&str> = syl.iter().map(|s| s.nucleus.ipa.as_str()).collect();
    assert_eq!(
        nuclei,
        ["ɛ", "ɛ", "ɛ", "a", "u", "a", "i", "ɪ", "i", "a", "a", "i", "ɔ", "a"]
    );
    assert_eq!(lyric_language(line, None).unwrap(), LyricLang::Uk);
    assert_eq!(lyric_language("Во поле", None).unwrap(), LyricLang::Ru);
    assert_eq!(lyric_language("hello", None).unwrap(), LyricLang::En);
}

#[test]
fn a_mark_beats_the_list_and_hyphens_are_the_sung_breaks() {
    let marked = sing_lyrics("У-кра-ї-ни", Some("uk"), &EnglishG2P::new()).unwrap();
    assert_eq!(marked.len(), 4);
    assert_eq!(marked[2].nucleus.ipa, "i");
    assert_eq!(ipa("воля́"), "v ɔ lʲ a");
    assert_eq!(stress_of("воля́")[3], 1);
}
