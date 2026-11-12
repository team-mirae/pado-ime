//! The engine on the shipped data files.

use std::sync::Arc;

use bada::engine::Mode;
use bada::{Data, Engine, Key, Mods};

fn data() -> Arc<Data> {
    let read = |f: &str| {
        std::fs::read_to_string(format!("{}/data/{f}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    };
    Arc::new(
        Data::parse(
            &read("ko-words.tsv"),
            &read("en-words.txt"),
            &read("hanja.tsv"),
            &read("oecd.tsv"),
        )
        .with_japanese(&read("ja-words.txt"))
        .with_kanji(&std::fs::read(format!("{}/data/kanji.bin", env!("CARGO_MANIFEST_DIR"))).unwrap())
        .with_lm(&std::fs::read(format!("{}/data/ko-jamo-4gram.bin", env!("CARGO_MANIFEST_DIR"))).unwrap())
        .with_search(&read("symbols.tsv"), &read("emoji.tsv"), &read("unicode-names.tsv")),
    )
}

fn typed(e: &mut Engine, keys: &[(Key, bool)]) -> bada::Output {
    let mut out = bada::Output::default();
    for &(k, shift) in keys {
        out = e.key(k, if shift { Mods::SHIFT } else { Mods::NONE });
    }
    out
}

/// 감사합니다 on layout 13: ㄱ(D) ㅏ(H) ㅁ(G) ㅅ(R) ㅏ ㅎ(V) ㅏ ㅂ(B) ㄴ(E) ㅣ(M) ㄷ(A) ㅏ
const GAMSAHAMNIDA: [Key; 12] = {
    use Key::*;
    [KeyD, KeyH, KeyG, KeyR, KeyH, KeyV, KeyH, KeyB, KeyE, KeyM, KeyA, KeyH]
};

/// A word and its particle (spec §7.0): an English word on QWERTY, then a Korean or Japanese
/// particle on another layout of the set, in one staged run.
#[test]
fn a_word_and_its_particle() {
    let d = data();
    let shown = |ids: &[u32], keys: &[(Key, bool)]| {
        let mut e = Engine::new(d.clone());
        e.pin_dictionary(None);
        let mut s = bada::Settings::default();
        s.enabled = ids.to_vec();
        e.set_settings(s);
        e.set_mode(Mode::Henkan);
        typed(&mut e, keys)
    };
    let letters = |text: &str| -> Vec<(Key, bool)> {
        text.chars()
            .map(|c| match c {
                '@' => (Key::BracketLeft, false),
                c => (Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), c.is_ascii_uppercase()),
            })
            .collect()
    };
    // bada plain (16): 에 is B, 를 I S I, 는 L S L, 에서 B O A
    assert_eq!(shown(&[0, 16], &letters("Claudeb")).preedit, "Claude에");
    assert_eq!(shown(&[0, 16], &letters("reportisi")).preedit, "report를");
    assert_eq!(shown(&[0, 16], &letters("APIisi")).preedit, "API를");
    assert_eq!(shown(&[0, 16], &letters("clientboa")).preedit, "client에서");
    // bada (13): 에 is F O
    assert_eq!(shown(&[0, 13], &letters("Claudefo")).preedit, "Claude에");
    // a Japanese particle after a name: で is W @; one key is offered, not shown (の is K)
    assert_eq!(shown(&[0, 20], &letters("GitHubw@")).preedit, "GitHubで");
    let out = shown(&[0, 20], &letters("Claudek"));
    assert_eq!(out.preedit, "Claudek");
    assert!(out.menu.iter().any(|m| m == "Claudeの"), "{:?}", out.menu);
    // an English word stays whole, and so does Korean
    assert_eq!(shown(&[0, 16], &letters("then")).preedit, "then");
    assert_eq!(shown(&[0, 16, 20], &letters("PowerPoint")).preedit, "PowerPoint");
    assert_eq!(shown(&[0, 16], &letters("hflkek")).preedit, "한국");
    // without QWERTY there is no such reading
    assert_ne!(shown(&[16], &letters("Claudeb")).preedit, "Claude에");
}

/// The default henkan candidate is bada plain (16; docs/layouts/16-bada-plain.md): vowels under
/// the left hand, consonants under the right.
#[test]
fn bada_plain_is_the_default() {
    use Key::*;
    let typed_on_default = |keys: &[(Key, bool)]| {
        let mut e = Engine::new(data());
        assert_eq!(e.settings().enabled, [16]);
        e.set_mode(Mode::Henkan);
        typed(&mut e, keys).preedit
    };
    // 감사합니다: ㄱ(K) ㅏ(F) ㅁ(P) ㅅ(O) ㅏ ㅎ(H) ㅏ ㅂ(N) ㄴ(L) ㅣ(D) ㄷ(;) ㅏ
    let plain = [KeyK, KeyF, KeyP, KeyO, KeyF, KeyH, KeyF, KeyN, KeyL, KeyD, Semicolon, KeyF].map(|k| (k, false));
    assert_eq!(typed_on_default(&plain), "감사합니다");
    // 한국어: the last ㅓ with ⇧ starts its own syllable on ㅇ; without it, it takes the ㄱ (한구거)
    let mut hangugeo = [KeyH, KeyF, KeyL, KeyK, KeyE, KeyK, KeyA].map(|k| (k, false));
    assert_eq!(typed_on_default(&hangugeo), "한구거");
    hangugeo[6].1 = true;
    assert_eq!(typed_on_default(&hangugeo), "한국어");
    // ⇧ + a consonant is it and ㅏ: ⇧ㄱ ⇧ㅈ ㅗ ㄱ = 가족
    assert_eq!(typed_on_default(&[(KeyK, true), (KeyU, false), (KeyW, false), (KeyK, false)]), "가족");
}

#[test]
fn bada_on_real_words() {
    use Key::*;
    let d = data();
    let plain = |keys: &[Key]| {
        let mut e = Engine::new(d.clone());
        let mut s = bada::Settings::default();
        s.enabled = vec![13];
        e.set_settings(s);
        e.set_mode(Mode::Henkan);
        typed(&mut e, &keys.iter().map(|&k| (k, false)).collect::<Vec<_>>())
    };
    assert_eq!(plain(&GAMSAHAMNIDA).preedit, "감사합니다");
    // 있습니다: ㅇ(F) ㅣ(M) ㅆ(') ㅅ(R) ㅡ(,) ㅂ(B) ㄴ(E) ㅣ ㄷ(A) ㅏ
    assert_eq!(plain(&[KeyF, KeyM, Quote, KeyR, Comma, KeyB, KeyE, KeyM, KeyA, KeyH]).preedit, "있습니다");
    // consonants alone stay as typed
    assert_eq!(plain(&[KeyX, KeyX, KeyX]).preedit, "ㅋㅋㅋ");
    assert_eq!(plain(&[KeyD, KeyR]).preedit, "ㄱㅅ");
}

#[test]
fn romaji_beside_bada() {
    let mut e = Engine::new(data());
    let mut s = bada::Settings::default();
    s.enabled = vec![13, 21];
    e.set_settings(s);
    // konnichiha: a known Japanese word beats bada's reading of the same keys
    let keys: Vec<(Key, bool)> = "konnichiha"
        .chars()
        .map(|c| {
            (
                Key::from_code(&format!("Key{}", c.to_ascii_uppercase())),
                false,
            )
        })
        .collect();
    let out = typed(&mut e, &keys);
    eprintln!("konnichiha → {} / {:?}", out.preedit, out.menu);
    // a Japanese reading wins; its kanji conversion and the kana are listed, with notes
    // (script forms off, the default: katakana is a ranked entry, not −2)
    assert!(out.menu.contains(&"こんにちは".to_string()), "{:?}", out.menu);
    assert!(out.menu.contains(&"コンニチハ".to_string()), "{:?}", out.menu);
    assert_eq!(out.zero, 0);
    let kana_row = out.menu.iter().position(|m| m == "こんにちは").unwrap();
    assert_eq!(out.notes[kana_row], "konnichiha");
    // and bada still wins on Korean
    let mut e = Engine::new(data());
    let mut s = bada::Settings::default();
    s.enabled = vec![13, 21];
    e.set_settings(s);
    let out = typed(&mut e, &GAMSAHAMNIDA.iter().map(|&k| (k, false)).collect::<Vec<_>>());
    assert_eq!(out.preedit, "감사합니다");
}

#[test]
fn kana_korean_offers_annyeong() {
    use Key::*;
    let mut e = Engine::new(data());
    let mut s = bada::Settings::default();
    s.enabled = vec![19];
    e.set_settings(s);
    let out = typed(
        &mut e,
        &[
            (Digit3, false),
            (KeyY, false),
            (KeyI, false),
            (Digit9, true),
            (KeyY, true),
            (KeyH, false),
            (BracketLeft, false),
            (KeyF, false),
            (KeyP, false),
            (Digit9, false),
        ],
    );
    eprintln!("{} / {:?}", out.preedit, out.menu);
    assert_eq!(out.preedit, "안뇽구하세요");
    assert_eq!(out.menu.get(1).map(String::as_str), Some("안녕하세요"));
}

#[test]
fn korean_roman_on_real_words() {
    let mut e = Engine::new(data());
    let mut s = bada::Settings::default();
    s.enabled = vec![18];
    e.set_settings(s);
    let letters = |w: &str| w.chars().map(|c| (Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), false)).collect::<Vec<_>>();
    let out = typed(&mut e, &letters("annyeonghaseyo"));
    assert_eq!(out.preedit, "안녕하세요");
    let mut e = Engine::new(data());
    let mut s = bada::Settings::default();
    s.enabled = vec![18];
    e.set_settings(s);
    let started = std::time::Instant::now();
    let out = typed(&mut e, &letters("gamsahamnida"));
    eprintln!("{} / {:?} in {:?}", out.preedit, out.menu, started.elapsed());
    assert_eq!(out.preedit, "감사함니다");
    assert!(out.menu.contains(&"감사합니다".to_string()));
}

#[test]
fn kanji_conversion() {
    let d = data();
    let kanji = d.kanji.as_ref().expect("kanji.bin");
    // Mozc's evaluation set: the floors JIS Moonshot holds
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/kanji-evaluation.tsv")).unwrap();
    let (mut total, mut exact, mut listed) = (0, 0, 0);
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let fields: Vec<&str> = line.split('\t').collect();
        let [_, input, expected, command, ..] = fields[..] else { continue };
        if !command.starts_with("Conversion") {
            continue;
        }
        total += 1;
        let got = kanji.convert(input, 3);
        exact += (got.first().map(|g| g.0.as_str()) == Some(expected)) as usize;
        listed += got.iter().any(|g| g.0 == expected) as usize;
    }
    eprintln!("kanji: {exact}/{total} exact, {listed}/{total} in the top 3");
    assert!(exact * 100 >= total * 66 && listed * 100 >= total * 75);

    // in the engine: わたしはがくせいです (romaji) → 私は学生です on top, its kana as the note
    let mut e = Engine::new(d);
    let mut s = bada::Settings::default();
    s.enabled = vec![21];
    e.set_settings(s);
    let keys: Vec<(Key, bool)> = "watashihagakuseidesu".chars().map(|c| (Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), false)).collect();
    let out = typed(&mut e, &keys);
    eprintln!("{} / {:?} / {:?}", out.preedit, out.menu, out.notes);
    assert_eq!(out.preedit, "私は学生です");
    assert_eq!(out.notes[out.selected], "わたしはがくせいです");
}

#[test]
fn the_example_dictionary_loads() {
    let json = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/dictionary-example.json")).unwrap();
    let (_, summary) = bada::userdict::UserDict::parse(&json);
    assert!(summary.ok, "{}", summary.error);
    assert_eq!((summary.positive, summary.negative), (5, 3));
}

#[test]
fn typo_suggestions_stay_under_the_typed_reading() {
    use Key::*;
    let d = data();
    let run = |keys: &[(Key, bool)]| {
        let mut e = Engine::new(d.clone());
        let mut s = bada::Settings::default();
        s.enabled = vec![13];
        e.set_settings(s);
        e.set_mode(Mode::Henkan);
        typed(&mut e, keys)
    };
    let plain = |ks: &[Key]| ks.iter().map(|&k| (k, false)).collect::<Vec<_>>();
    // 감사합니다 with the final-only K (ㄴ) where E (initial ㄴ) was meant: 감사합ㄴㅣ다 shows first,
    // 감사합니다 is suggested under it
    let out = run(&plain(&[KeyD, KeyH, KeyG, KeyR, KeyH, KeyV, KeyH, KeyB, KeyK, KeyM, KeyA, KeyH]));
    eprintln!("{} / {:?} / {:?}", out.preedit, out.menu, out.notes);
    assert_ne!(out.preedit, "감사합니다");
    assert_eq!(out.menu[0], out.preedit);
    assert!(out.menu[1..].contains(&"감사합니다".to_string()), "{:?}", out.menu);
    // typed right: nothing to suggest
    let out = run(&plain(&GAMSAHAMNIDA));
    assert_eq!(out.preedit, "감사합니다");
    assert!(out.menu.is_empty(), "{:?}", out.menu);
}

#[test]
fn symbols_and_unicode_on_real_data() {
    let d = data();
    let all: Vec<String> = bada::search::TONES.iter().map(|t| t.to_string()).collect();
    let texts = |q: &str| d.search.symbols(q, &all, 12).into_iter().map(|x| x.0).collect::<Vec<_>>();
    eprintln!("happy face → {:?}", texts("happy face"));
    eprintln!("¥ → {:?}", texts("¥"));
    eprintln!(". → {:?}", texts("."));
    eprintln!("thumbs → {:?}", texts("thumbs"));
    assert!(texts("happy face").contains(&"😀".to_string()));
    assert_eq!(texts("usd")[0], "$");
    assert_eq!(texts("¥")[..5], ["$", "€", "₩", "¥", "£"]);
    assert!(texts("dollar").contains(&"$".to_string()));
    let t = std::time::Instant::now();
    let snow = d.search.unicode("snowman", 10);
    eprintln!("snowman → {:?} in {:?}", snow.iter().map(|x| &x.0).collect::<Vec<_>>(), t.elapsed());
    assert!(snow.iter().any(|x| x.0 == "☃"));
    assert_eq!(d.search.unicode("u+ac00", 3)[0].0, "가");
}


#[test]
fn colloquial_unicode_on_real_data() {
    let d = data();
    let texts = |q: &str| d.search.unicode(q, 6).into_iter().map(|x| x.0).collect::<Vec<_>>();
    eprintln!("ju {:?} · joo {:?} · corporation {:?} · pyeong {:?} · pyung {:?} · kabu {:?} · yuubin {:?}",
        texts("ju"), texts("joo"), texts("corporation"), texts("pyeong"), texts("pyung"), texts("kabu"), texts("yuubin"));
    assert_eq!(texts("joo")[0], "㈜");
    assert_eq!(texts("ju")[0], "㈜");
    assert_eq!(texts("pyeong")[..2], ["평", "坪"]);
    assert_eq!(texts("pyung")[..2], ["평", "坪"]);
    assert_eq!(texts("kabu")[0], "㈱");
    assert_eq!(texts("yuubin")[0], "〒");
    // Symbols (150) doesn't offer them, so a profile with both lists each once
    let all: Vec<String> = bada::search::TONES.iter().map(|t| t.to_string()).collect();
    assert!(!d.search.symbols("joo", &all, 10).iter().any(|x| x.0 == "㈜"));
}

/// Kana-native words (docs/spec.md §11.2, "Where the kana goes"): their kana first, also inside a
/// phrase; a known word's kana no lower than second.
#[test]
fn kana_native_words_stay_kana() {
    let romaji = |letters: &str| {
        let mut e = Engine::new(data());
        e.pin_dictionary(None);
        let mut s = bada::Settings::default();
        s.enabled = vec![21];
        e.set_settings(s);
        // ' is ⇧7 on JIS.
        let keys: Vec<(Key, bool)> = letters
            .chars()
            .map(|c| if c == '\'' { (Key::Digit7, true) } else { (Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), false) })
            .collect();
        typed(&mut e, &keys)
    };
    let out = romaji("kono");
    assert_eq!(out.preedit, "この", "{:?}", out.menu);
    let out = romaji("chotto");
    assert_eq!(out.preedit, "ちょっと", "{:?}", out.menu);
    let out = romaji("konohon");
    assert_eq!(out.preedit, "この本", "{:?}", out.menu);
    let out = romaji("anata");
    assert!(out.menu[out.zero..].iter().take(2).any(|m| m == "あなた"), "{:?}", out.menu);
    // kanji words stay kanji
    let out = romaji("nihongo");
    assert_eq!(out.preedit, "日本語", "{:?}", out.menu);
    // Common words (ja-common-words.txt) are in the first rows when their reading is typed alone,
    // and phrases are converted as before.
    for (letters, word) in [("ichi", "一"), ("shi", "死"), ("shi", "詩"), ("ki", "木"), ("gen'in", "原因")] {
        let out = romaji(letters);
        assert!(out.menu[out.zero..].iter().take(5).any(|m| m == word), "{letters}: {:?}", out.menu);
    }
    // ... under a known word's kana, which stays no lower than second (ひ: 日, ひ, 火).
    let out = romaji("hi");
    assert_eq!(&out.menu[out.zero..out.zero + 3], ["日", "ひ", "火"], "{:?}", out.menu);
    let out = romaji("toukyouni");
    assert_eq!(out.preedit, "東京に", "{:?}", out.menu);
}

/// The user's Japanese words count in whatever script they're written (docs/dictionary-contract.md):
/// the kana typed, its katakana or a conversion of it, also one the menu wouldn't have listed.
#[test]
fn japanese_dictionary_words_in_kana_katakana_and_kanji() {
    let d = data();
    let engine = |layouts: &[u32], words: &[(&str, u32)]| {
        let positive: Vec<String> =
            words.iter().map(|(w, n)| format!(r#"{{"word": "{w}", "language": "japanese", "count": {n}}}"#)).collect();
        let json = format!(r#"{{"format": "bada-dictionary", "version": 1, "positive": [{}]}}"#, positive.join(", "));
        let (dict, summary) = bada::userdict::UserDict::parse(&json);
        assert!(summary.ok, "{}", summary.error);
        let mut e = Engine::new(d.clone());
        e.pin_dictionary((!words.is_empty()).then(|| Arc::new(dict)));
        let mut s = bada::Settings::default();
        s.enabled = layouts.to_vec();
        e.set_settings(s);
        e
    };
    let romaji = |layouts: &[u32], words: &[(&str, u32)], letters: &str| {
        let keys: Vec<(Key, bool)> =
            letters.chars().map(|c| (Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), false)).collect();
        typed(&mut engine(layouts, words), &keys)
    };
    let rows = |out: &bada::Output| out.menu[out.zero..].to_vec();
    // katakana: クバネテス isn't what conversion makes of くばねてす
    assert_eq!(romaji(&[21], &[], "kubanetesu").preedit, "区バネテス");
    let out = romaji(&[21], &[("クバネテス", 1)], "kubanetesu");
    assert_eq!(out.preedit, "クバネテス", "{:?}", out.menu);
    assert_eq!(out.notes[out.selected], "くばねてす");
    // hiragana: first, not second
    assert_eq!(romaji(&[21], &[("くばねてす", 1)], "kubanetesu").preedit, "くばねてす");
    // kanji: the second conversion comes first
    assert_eq!(romaji(&[21], &[], "satou").preedit, "佐藤");
    assert_eq!(romaji(&[21], &[("砂糖", 1)], "satou").preedit, "砂糖");
    // kanji the menu doesn't list (正幸 is far down the conversions of せいこう)
    let out = romaji(&[21], &[], "seikou");
    assert!(!rows(&out).contains(&"正幸".to_string()), "{:?}", out.menu);
    assert_eq!(romaji(&[21], &[("正幸", 1)], "seikou").preedit, "正幸");
    // several words of one reading: the most used first
    let out = romaji(&[21], &[("砂糖", 5), ("サトウ", 9)], "satou");
    assert_eq!(rows(&out)[..2], ["サトウ", "砂糖"], "{:?}", out.menu);
    // its reading counts as a known word beside the other layouts: たけ is no known word, so
    // English `take` shows; with 竹 in the dictionary, 竹 does
    assert_eq!(romaji(&[0, 21], &[], "take").preedit, "take");
    assert_eq!(romaji(&[0, 21], &[("竹", 1)], "take").preedit, "竹");
    // the kana layout too: さ と う
    let kana = [(Key::KeyX, false), (Key::KeyS, false), (Key::Digit4, false)];
    assert_eq!(typed(&mut engine(&[20], &[]), &kana).preedit, "佐藤");
    assert_eq!(typed(&mut engine(&[20], &[("砂糖", 1)]), &kana).preedit, "砂糖");
    // script forms on: the user's katakana is still a ranked row, first
    let mut e = engine(&[21], &[("サトウ", 1)]);
    let mut s = e.settings().clone();
    s.script_forms = true;
    e.set_settings(s);
    let keys: Vec<(Key, bool)> = "satou".chars().map(|c| (Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), false)).collect();
    assert_eq!(typed(&mut e, &keys).preedit, "サトウ");
}

/// Layout 15 on the shipped dictionary: the spelling's reading wins unless another is far better known
/// (jong 종, not 적), and with no known word the jamo model picks the split (지역총괄의, 종속기업이).
#[test]
fn bada_roman_ranks_the_spelling_first() {
    let d = data();
    for (typed, want) in [("jong", "종"), ("jongsok", "종속"), ("jongsokgieob", "종속기업"), ("jongsokgieobi", "종속기업이"),
                          ("sok", "속"), ("eui", "의"), ("its", "있"), ("itseo", "있어"), ("gaeui", "개의"), ("gwalui", "괄의"), ("jiyeokchonggwalui", "지역총괄의")] {
        let mut e = Engine::new(d.clone());
        e.pin_dictionary(None);
        let mut s = bada::Settings::default();
        s.enabled = vec![15];
        e.set_settings(s);
        let keys: Vec<(Key, bool)> = typed.chars().map(|c| (Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), false)).collect();
        assert_eq!(typed_out(&mut e, &keys), want, "{typed}");
    }
}

fn typed_out(e: &mut Engine, keys: &[(Key, bool)]) -> String {
    typed(e, keys).preedit
}

/// In a set the key models compare the layouts (docs/spec.md §7.0): what is typed on one layout's
/// keys still shows, whichever of the layout's ways a person types it.
#[test]
fn a_set_shows_the_layout_that_was_typed() {
    let d = data();
    let shown = |layouts: &[u32], keys: &[(Key, bool)]| {
        let mut e = Engine::new(d.clone());
        e.pin_dictionary(None);
        let mut s = bada::Settings::default();
        s.enabled = layouts.to_vec();
        e.set_settings(s);
        typed_out(&mut e, keys)
    };
    let letters = |w: &str| -> Vec<(Key, bool)> {
        w.chars().map(|c| (Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), c.is_ascii_uppercase())).collect()
    };
    // English beside Korean: common words that are Korean words on the other layout's keys
    // (the 판, to 페, and 덕 on 13), a capital, a word no list has
    for sets in [&[0, 13][..], &[0, 10], &[0, 13, 20], &[0, 15, 21]] {
        for word in ["the", "to", "and", "The", "kubernetes"] {
            assert_eq!(shown(sets, &letters(word)), word, "{sets:?}");
        }
    }
    // Korean on 13 beside English and kana, with the final-only keys (bada::typo::bada_keys: ㄴ on K,
    // ㅇ on J, ㄹ on L) and with the keys that are an initial or a final (ㄴ on E, ㄹ on C)
    for word in ["한국", "사람들이", "있습니다", "그리고", "안녕하세요", "생각합니다"] {
        let keys: Vec<(Key, bool)> = bada::typo::bada_keys(word).unwrap().iter().map(|p| (p.key, p.shift)).collect();
        for sets in [&[0, 13][..], &[13, 20], &[0, 13, 20], &[13, 21]] {
            assert_eq!(shown(sets, &keys), word, "{sets:?}");
        }
    }
    use Key::*;
    let plain = |keys: &[Key]| keys.iter().map(|&k| (k, false)).collect::<Vec<_>>();
    assert_eq!(shown(&[0, 13], &plain(&[KeyV, KeyH, KeyE, KeyD, Slash, KeyD])), "한국");
    assert_eq!(shown(&[0, 13], &plain(&[KeyV, KeyH, KeyK, KeyD, Slash, Semicolon])), "한국");
    // A full stop after a short Korean word doesn't turn it English (with 18 every key is staged)
    let stop = |w: &str| letters(w).into_iter().chain([(Period, false)]).collect::<Vec<_>>();
    assert_eq!(shown(&[0, 18], &stop("na")), "나.");
    assert_eq!(shown(&[0, 18], &stop("hanguk")), "한국.");
    assert_eq!(shown(&[0, 18], &stop("it")), "it.");
    // Japanese in romaji beside English and Korean: Hepburn, ん as one n, kunrei (but `sinbun`
    // beside 15, where it spells 신분)
    for sets in [&[0, 21][..], &[13, 21], &[0, 15, 21]] {
        for (typed, want) in [("nihongo", "日本語"), ("benkyou", "勉強"), ("shinbun", "新聞"), ("sinbun", "新聞"), ("konnichiha", "こんにちは")] {
            if typed != "sinbun" || !sets.contains(&15) {
                assert_eq!(shown(sets, &letters(typed)), want, "{typed} {sets:?}");
            }
        }
    }
}

/// QWERTY and layout 15 together: a real English word that's no Korean word types as English
/// (interpolate); Korean that's a word stays Korean.
#[test]
fn english_beside_bada_roman() {
    let d = data();
    // (love, told: common words from the generated part of en-words.txt, `word<TAB>count`)
    for (typed, want) in [("interpolate", "interpolate"), ("serendipity", "serendipity"), ("hanguk", "한국"),
                          ("saram", "사람"), ("jongsok", "종속"), ("sarang", "사랑"), ("gamsa", "감사"),
                          ("love", "love"), ("told", "told")] {
        let mut e = Engine::new(d.clone());
        e.pin_dictionary(None);
        let mut s = bada::Settings::default();
        s.enabled = vec![0, 15];
        e.set_settings(s);
        let keys: Vec<(Key, bool)> = typed.chars().map(|c| (Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), false)).collect();
        assert_eq!(typed_out(&mut e, &keys), want, "{typed}");
    }
}
