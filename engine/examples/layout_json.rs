//! Write docs/layouts/NN-name.json for the layouts the engine defines in code (0, 10, 11, 12, 15, 16,
//! 18, 20, 21, 150, 151), in the editor's `bada-layout/1` format plus the fields the typing robot
//! reads (`languages`, `input`, `rules`, spelling tables; docs/robot.md).
//!
//!     cargo run --example layout_json
//!
//! 13 and 14 are written by hand (13 in the editor, 14 is read by the engine), as is 19's. The
//! robot test checks every file against the engine (`json_matches_engine` in tests/robot.rs).

use bada::Key;
use bada::layout::{self, Role};
use serde_json::{Value, json};

fn tok(t: impl Into<String>, r: &str) -> Value {
    json!({"t": t.into(), "r": r})
}

/// A Korean role as tokens. Two tokens on one key are alternatives (an initial or a final, chosen
/// by the composer's state).
fn role_tokens(r: Role) -> Vec<Value> {
    match r {
        Role::Cons { ini: Some(a), fin: Some(b) } if a == b => vec![tok(a, "c")],
        Role::Cons { ini, fin } => {
            let mut v = vec![];
            if let Some(a) = ini {
                v.push(tok(a, "i"));
            }
            if let Some(b) = fin {
                v.push(tok(b, "f"));
            }
            v
        }
        Role::ConsA(c) => vec![tok(c, "a")],
        Role::Vowel(x) => vec![tok(x, "v")],
        Role::VowelNew(x) => vec![tok(x, "n")],
        Role::Lit(c) => vec![tok(c, "x")],
    }
}

fn keys(f: impl Fn(Key, bool) -> Vec<Value>) -> Value {
    let mut m = serde_json::Map::new();
    for &k in Key::ALL.iter().filter(|k| k.is_kana_space()) {
        m.insert(k.code().into(), json!({"base": f(k, false), "shift": f(k, true)}));
    }
    Value::Object(m)
}

const ROLES: &str = r#"{"i":"초성","v":"중성","f":"종성","c":"초성+종성","a":"초성+ㅏ","n":"ㅇ+중성","x":"text","k":"kana","l":"letter"}"#;

/// `languages`: each language the layout types, with the reliability it claims for it
/// (perfect, production, experimental; docs/robot.md). The robot test holds it to the claim.
fn layout(id: u32, name: &str, languages: &[(&str, &str)], input: &str, keys: Value) -> Value {
    let reliability: serde_json::Map<String, Value> = languages.iter().map(|(l, r)| (l.to_string(), json!(r))).collect();
    json!({
        "format": "bada-layout/1", "id": id, "name": name, "keyboard": "hhkb",
        "languages": languages.iter().map(|l| l.0).collect::<Vec<_>>(), "reliability": reliability, "input": input,
        "roles": serde_json::from_str::<Value>(ROLES).unwrap(),
        "keys": keys,
    })
}

fn korean(id: u32, name: &str) -> Value {
    let r = layout::rules(id).unwrap();
    let mut v = layout(id, name, &[("korean", "perfect")], "compose", keys(|k, s| layout::role(id, k, s).map(role_tokens).unwrap_or_default()));
    v["implied_ieung"] = json!(r.implied_ieung);
    v["rules"] = json!({"join_vowels": r.join_vowels, "implied_ieung": r.implied_ieung, "double_toggle": r.double_toggle,
                        "held_shift": r.held_shift, "double_tense": r.double_tense});
    v
}

fn jis(k: Key, s: bool, letter: &str) -> Vec<Value> {
    match layout::jis_char(k, s) {
        Some(c) if c.is_ascii_alphabetic() => vec![tok(c, letter)],
        Some(c) => vec![tok(c, "x")],
        None => vec![],
    }
}

/// Kana → the romaji the robot types for it (Hepburn as Japanese IMEs spell it; romaji.rs).
fn romaji_table() -> Value {
    let mut m = serde_json::Map::new();
    let small = ['ゃ', 'ゅ', 'ょ', 'ぁ', 'ぃ', 'ぅ', 'ぇ', 'ぉ'];
    for c in ('ぁ'..='ゔ').chain(['ー']) {
        if matches!(c, 'っ' | 'ん') {
            continue;
        }
        if let Some(r) = bada::romaji::romaji(&c.to_string()) {
            m.insert(c.to_string(), json!(r));
        }
        for s in small {
            let pair = format!("{c}{s}");
            if let Some(r) = bada::romaji::romaji(&pair) {
                if r.len() <= 3 && !r.starts_with('x') && !r.contains('x') {
                    m.insert(pair, json!(r));
                }
            }
        }
    }
    // Spellings the notes don't use but the IME does (romaji.rs reads ぢ as ji, てぃ as ti).
    for (k, r) in [("ぢ", "di"), ("づ", "du"), ("てぃ", "thi"), ("でぃ", "dhi"), ("てゅ", "thu"), ("でゅ", "dhu"),
                   ("ぢゃ", "dya"), ("ぢゅ", "dyu"), ("ぢょ", "dyo"), ("ゐ", "wyi"), ("ゑ", "wye")] {
        m.insert(k.into(), json!(r));
    }
    // Each spelling must type its kana in the engine (else it's left out and reported).
    m.retain(|kana, r| {
        let presses: Vec<_> = r.as_str().unwrap().chars()
            .map(|c| bada::layout::Press::new(letter_key(c), false)).collect();
        let ok = bada::japanese::romaji_compose(&presses) == *kana;
        if !ok && kana.chars().count() == 1 {
            eprintln!("romaji: {} {kana} gives {}", r, bada::japanese::romaji_compose(&presses));
        }
        ok
    });
    m.insert("ん".into(), json!("nn"));
    // ん before a vowel, y or n (ぜんいん zen'in): after `nn` a vowel starts the next kana.
    m.insert("ん'".into(), json!("n'"));
    m.insert("っ".into(), json!("xtu"));
    Value::Object(m)
}

fn letter_key(c: char) -> Key {
    match c {
        '-' => Key::Minus,
        '\'' => Key::Digit7,
        _ => Key::from_code(&format!("Key{}", c.to_ascii_uppercase())),
    }
}

/// Revised Romanization, letter by letter (no sound changes): layout 18's first spellings.
fn roman_table() -> Value {
    json!({
        "onsets": {"ㄱ":"g","ㄲ":"kk","ㄴ":"n","ㄷ":"d","ㄸ":"tt","ㄹ":"r","ㅁ":"m","ㅂ":"b","ㅃ":"pp","ㅅ":"s","ㅆ":"ss",
                   "ㅇ":"","ㅈ":"j","ㅉ":"jj","ㅊ":"ch","ㅋ":"k","ㅌ":"t","ㅍ":"p","ㅎ":"h"},
        "vowels": {"ㅏ":"a","ㅐ":"ae","ㅑ":"ya","ㅒ":"yae","ㅓ":"eo","ㅔ":"e","ㅕ":"yeo","ㅖ":"ye","ㅗ":"o","ㅘ":"wa","ㅙ":"wae",
                   "ㅚ":"oe","ㅛ":"yo","ㅜ":"u","ㅝ":"wo","ㅞ":"we","ㅟ":"wi","ㅠ":"yu","ㅡ":"eu","ㅢ":"ui","ㅣ":"i"},
        "codas": {"ㄱ":"k","ㄲ":"kk","ㄳ":"ks","ㄴ":"n","ㄵ":"nj","ㄶ":"nh","ㄷ":"d","ㄹ":"l","ㄺ":"lk","ㄻ":"lm","ㄼ":"lb",
                  "ㅀ":"lh","ㅁ":"m","ㅂ":"p","ㅄ":"ps","ㅅ":"s","ㅆ":"ss","ㅇ":"ng","ㅈ":"t","ㅊ":"t","ㅋ":"k","ㅌ":"t",
                  "ㅍ":"p","ㅎ":"t"},
        "break": "-"
    })
}

fn write(name: &str, v: &Value) {
    let path = format!("{}/../docs/layouts/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::write(&path, serde_json::to_string(v).unwrap() + "\n").unwrap();
    println!("{name}");
}

fn main() {
    write("00-qwerty.json", &layout(0, "JIS QWERTY", &[("english", "perfect")], "direct", keys(|k, s| jis(k, s, "l"))));
    write("10-kts.json", &korean(layout::KTS, "JIS KTS"));
    write("11-korean-legacy.json", &korean(layout::LEGACY_KOREAN, "JIS Korean Legacy"));
    write("12-moonshot.json", &korean(layout::LEGACY_MOONSHOT, "JIS Moonshot"));
    write("16-bada-plain.json", &korean(layout::BADA_PLAIN, "bada plain"));

    // 15: the letters as 18, with its own keys (q ch, x an initial ㅇ, `;` the break, ¥ ₩, ⇧0 +, ⇧[ 「, ⇧] 」).
    let mut bada_roman = layout(15, "bada roman", &[("korean", "production")], "roman",
        keys(|k, s| bada::bada_roman::token(k, s).map(|(t, r)| vec![tok(t, r)]).unwrap_or_default()));
    let mut table = roman_table();
    table["break"] = json!(";");
    bada_roman["spelling"] = table;
    write("15-bada-roman.json", &bada_roman);

    let mut roman = layout(18, "JIS Korean Roman", &[("korean", "production")], "roman", keys(|k, s| jis(k, s, "l")));
    roman["spelling"] = roman_table();
    write("18-korean-roman.json", &roman);

    let kana = |k: Key, s: bool| match bada::japanese::jis_kana(k, s) {
        Some(c) if ('ぁ'..='ゖ').contains(&c) || matches!(c, 'ー' | '゛' | '゜') => vec![tok(c, "k")],
        Some(c) => vec![tok(c, "x")],
        None => vec![],
    };
    let mut kana20 = layout(20, "JIS Kana", &[("japanese_kana", "production")], "kana", keys(kana));
    kana20["marks"] = json!({"゛": "voiced", "゜": "semi-voiced"});
    write("20-kana.json", &kana20);

    let mut romaji = layout(21, "JIS Romaji", &[("japanese_romaji", "production")], "romaji", keys(|k, s| jis(k, s, "l")));
    romaji["spelling"] = romaji_table();
    write("21-romaji.json", &romaji);

    write("150-symbols.json", &layout(150, "JIS Symbols", &[], "search", keys(|k, s| jis(k, s, "l"))));
    write("151-unicode.json", &layout(151, "JIS Unicode", &[], "search", keys(|k, s| jis(k, s, "l"))));
}
