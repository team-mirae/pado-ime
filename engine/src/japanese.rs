//! Japanese layouts, their kana readings: 20 JIS Kana (one kana per key, ゛ ゜ attach
//! to the kana before them) and 21 JIS Romaji (romaji to kana, as Japanese IMEs spell it).
//! Kanji conversion of the reading is in kanji.rs.
//! Kana tables and the dakuten automaton are ported from JIS Moonshot (engine/src/kana.rs).
//! Their script forms (spec §6.4: −1 hiragana, −2 katakana) and F-key actions (§6.5) are in
//! engine.rs. See docs/layouts/20-kana.md, docs/layouts/21-romaji.md and docs/spec.md §11.

use crate::key::Key;
use crate::layout::{Press, jis_char};
use crate::width::full_width;

pub const DAKUTEN: char = '\u{309B}';
pub const HANDAKUTEN: char = '\u{309C}';

/// The JIS kana layout: (unshifted, shifted).
pub fn jis_kana(key: Key, shift: bool) -> Option<char> {
    use Key::*;
    let (base, shifted) = match key {
        Digit1 => ('ぬ', 'ぬ'),
        Digit2 => ('ふ', 'ふ'),
        Digit3 => ('あ', 'ぁ'),
        Digit4 => ('う', 'ぅ'),
        Digit5 => ('え', 'ぇ'),
        Digit6 => ('お', 'ぉ'),
        Digit7 => ('や', 'ゃ'),
        Digit8 => ('ゆ', 'ゅ'),
        Digit9 => ('よ', 'ょ'),
        Digit0 => ('わ', 'を'),
        Minus => ('ほ', 'ほ'),
        Equal => ('へ', 'へ'),
        IntlYen => ('ー', 'ー'),
        KeyQ => ('た', 'た'),
        KeyW => ('て', 'て'),
        KeyE => ('い', 'ぃ'),
        KeyR => ('す', 'す'),
        KeyT => ('か', 'か'),
        KeyY => ('ん', 'ん'),
        KeyU => ('な', 'な'),
        KeyI => ('に', 'に'),
        KeyO => ('ら', 'ら'),
        KeyP => ('せ', 'せ'),
        BracketLeft => (DAKUTEN, DAKUTEN),
        BracketRight => (HANDAKUTEN, '「'),
        KeyA => ('ち', 'ち'),
        KeyS => ('と', 'と'),
        KeyD => ('し', 'し'),
        KeyF => ('は', 'は'),
        KeyG => ('き', 'き'),
        KeyH => ('く', 'く'),
        KeyJ => ('ま', 'ま'),
        KeyK => ('の', 'の'),
        KeyL => ('り', 'り'),
        Semicolon => ('れ', 'れ'),
        Quote => ('け', 'け'),
        Backslash => ('む', '」'),
        KeyZ => ('つ', 'っ'),
        KeyX => ('さ', 'さ'),
        KeyC => ('そ', 'そ'),
        KeyV => ('ひ', 'ひ'),
        KeyB => ('こ', 'こ'),
        KeyN => ('み', 'み'),
        KeyM => ('も', 'も'),
        Comma => ('ね', '、'),
        Period => ('る', '。'),
        Slash => ('め', '・'),
        IntlRo => ('ろ', 'ろ'),
        _ => return None,
    };
    Some(if shift { shifted } else { base })
}

fn voiced(c: char) -> Option<char> {
    match c {
        'う' => Some('ゔ'),
        'か' | 'き' | 'く' | 'け' | 'こ' | 'さ' | 'し' | 'す' | 'せ' | 'そ' | 'た' | 'ち'
        | 'つ' | 'て' | 'と' | 'は' | 'ひ' | 'ふ' | 'へ' | 'ほ' => {
            char::from_u32(c as u32 + 1)
        }
        _ => None,
    }
}

fn semi_voiced(c: char) -> Option<char> {
    match c {
        'は' | 'ひ' | 'ふ' | 'へ' | 'ほ' => char::from_u32(c as u32 + 2),
        _ => None,
    }
}

/// Layout 20: the kana of staged presses, ゛ ゜ attached where they can be.
pub fn kana_compose(presses: &[Press]) -> String {
    let mut out = String::new();
    for p in presses {
        let Some(c) = jis_kana(p.key, p.shift) else {
            continue;
        };
        let mark: Option<fn(char) -> Option<char>> = match c {
            DAKUTEN => Some(voiced),
            HANDAKUTEN => Some(semi_voiced),
            _ => None,
        };
        if let Some(mark) = mark {
            if let Some(marked) = out.chars().last().and_then(mark) {
                out.pop();
                out.push(marked);
                continue;
            }
        }
        out.push(c);
    }
    out
}

pub fn kana_takes(p: Press) -> bool {
    !p.alt && jis_kana(p.key, p.shift).is_some()
}

// ---------- romaji ----------

/// Romaji → hiragana, the common Japanese IME spellings (MS-IME / Kotoeri), longest first.
fn romaji_table() -> &'static [(&'static str, &'static str)] {
    &[
        ("ltsu", "っ"),
        ("xtsu", "っ"),
        ("kya", "きゃ"),
        ("kyi", "きぃ"),
        ("kyu", "きゅ"),
        ("kye", "きぇ"),
        ("kyo", "きょ"),
        ("gya", "ぎゃ"),
        ("gyi", "ぎぃ"),
        ("gyu", "ぎゅ"),
        ("gye", "ぎぇ"),
        ("gyo", "ぎょ"),
        ("sha", "しゃ"),
        ("shi", "し"),
        ("shu", "しゅ"),
        ("she", "しぇ"),
        ("sho", "しょ"),
        ("sya", "しゃ"),
        ("syi", "しぃ"),
        ("syu", "しゅ"),
        ("sye", "しぇ"),
        ("syo", "しょ"),
        ("zya", "じゃ"),
        ("zyi", "じぃ"),
        ("zyu", "じゅ"),
        ("zye", "じぇ"),
        ("zyo", "じょ"),
        ("jya", "じゃ"),
        ("jyi", "じぃ"),
        ("jyu", "じゅ"),
        ("jye", "じぇ"),
        ("jyo", "じょ"),
        ("cha", "ちゃ"),
        ("chi", "ち"),
        ("chu", "ちゅ"),
        ("che", "ちぇ"),
        ("cho", "ちょ"),
        ("cya", "ちゃ"),
        ("cyi", "ちぃ"),
        ("cyu", "ちゅ"),
        ("cye", "ちぇ"),
        ("cyo", "ちょ"),
        ("tya", "ちゃ"),
        ("tyi", "ちぃ"),
        ("tyu", "ちゅ"),
        ("tye", "ちぇ"),
        ("tyo", "ちょ"),
        ("tsa", "つぁ"),
        ("tsi", "つぃ"),
        ("tsu", "つ"),
        ("tse", "つぇ"),
        ("tso", "つぉ"),
        ("tha", "てゃ"),
        ("thi", "てぃ"),
        ("thu", "てゅ"),
        ("the", "てぇ"),
        ("tho", "てょ"),
        ("dya", "ぢゃ"),
        ("dyi", "ぢぃ"),
        ("dyu", "ぢゅ"),
        ("dye", "ぢぇ"),
        ("dyo", "ぢょ"),
        ("dha", "でゃ"),
        ("dhi", "でぃ"),
        ("dhu", "でゅ"),
        ("dhe", "でぇ"),
        ("dho", "でょ"),
        ("nya", "にゃ"),
        ("nyi", "にぃ"),
        ("nyu", "にゅ"),
        ("nye", "にぇ"),
        ("nyo", "にょ"),
        ("hya", "ひゃ"),
        ("hyi", "ひぃ"),
        ("hyu", "ひゅ"),
        ("hye", "ひぇ"),
        ("hyo", "ひょ"),
        ("bya", "びゃ"),
        ("byi", "びぃ"),
        ("byu", "びゅ"),
        ("bye", "びぇ"),
        ("byo", "びょ"),
        ("pya", "ぴゃ"),
        ("pyi", "ぴぃ"),
        ("pyu", "ぴゅ"),
        ("pye", "ぴぇ"),
        ("pyo", "ぴょ"),
        ("mya", "みゃ"),
        ("myi", "みぃ"),
        ("myu", "みゅ"),
        ("mye", "みぇ"),
        ("myo", "みょ"),
        ("rya", "りゃ"),
        ("ryi", "りぃ"),
        ("ryu", "りゅ"),
        ("rye", "りぇ"),
        ("ryo", "りょ"),
        ("xya", "ゃ"),
        ("xyu", "ゅ"),
        ("xyo", "ょ"),
        ("lya", "ゃ"),
        ("lyu", "ゅ"),
        ("lyo", "ょ"),
        ("xtu", "っ"),
        ("ltu", "っ"),
        ("xwa", "ゎ"),
        ("lwa", "ゎ"),
        ("ka", "か"),
        ("ki", "き"),
        ("ku", "く"),
        ("ke", "け"),
        ("ko", "こ"),
        ("ga", "が"),
        ("gi", "ぎ"),
        ("gu", "ぐ"),
        ("ge", "げ"),
        ("go", "ご"),
        ("sa", "さ"),
        ("si", "し"),
        ("su", "す"),
        ("se", "せ"),
        ("so", "そ"),
        ("za", "ざ"),
        ("zi", "じ"),
        ("zu", "ず"),
        ("ze", "ぜ"),
        ("zo", "ぞ"),
        ("ja", "じゃ"),
        ("ji", "じ"),
        ("ju", "じゅ"),
        ("je", "じぇ"),
        ("jo", "じょ"),
        ("ta", "た"),
        ("ti", "ち"),
        ("tu", "つ"),
        ("te", "て"),
        ("to", "と"),
        ("da", "だ"),
        ("di", "ぢ"),
        ("du", "づ"),
        ("de", "で"),
        ("do", "ど"),
        ("na", "な"),
        ("ni", "に"),
        ("nu", "ぬ"),
        ("ne", "ね"),
        ("no", "の"),
        ("ha", "は"),
        ("hi", "ひ"),
        ("hu", "ふ"),
        ("he", "へ"),
        ("ho", "ほ"),
        ("fa", "ふぁ"),
        ("fi", "ふぃ"),
        ("fu", "ふ"),
        ("fe", "ふぇ"),
        ("fo", "ふぉ"),
        ("ba", "ば"),
        ("bi", "び"),
        ("bu", "ぶ"),
        ("be", "べ"),
        ("bo", "ぼ"),
        ("pa", "ぱ"),
        ("pi", "ぴ"),
        ("pu", "ぷ"),
        ("pe", "ぺ"),
        ("po", "ぽ"),
        ("ma", "ま"),
        ("mi", "み"),
        ("mu", "む"),
        ("me", "め"),
        ("mo", "も"),
        ("ya", "や"),
        ("yi", "い"),
        ("yu", "ゆ"),
        ("ye", "いぇ"),
        ("yo", "よ"),
        ("ra", "ら"),
        ("ri", "り"),
        ("ru", "る"),
        ("re", "れ"),
        ("ro", "ろ"),
        ("wa", "わ"),
        ("wi", "うぃ"),
        ("wu", "う"),
        ("we", "うぇ"),
        ("wo", "を"),
        ("va", "ゔぁ"),
        ("vi", "ゔぃ"),
        ("vu", "ゔ"),
        ("ve", "ゔぇ"),
        ("vo", "ゔぉ"),
        ("xa", "ぁ"),
        ("xi", "ぃ"),
        ("xu", "ぅ"),
        ("xe", "ぇ"),
        ("xo", "ぉ"),
        ("la", "ぁ"),
        ("li", "ぃ"),
        ("lu", "ぅ"),
        ("le", "ぇ"),
        ("lo", "ぉ"),
        ("n'", "ん"),
        ("a", "あ"),
        ("i", "い"),
        ("u", "う"),
        ("e", "え"),
        ("o", "お"),
    ]
}

/// What one press types in romaji: a lowercase letter to compose, or text as it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RomajiInput {
    Letter(char),
    Text(char),
}

fn romaji_input(p: Press) -> Option<RomajiInput> {
    if p.alt {
        return None;
    }
    if p.key.is_letter() {
        let c = jis_char(p.key, false)?;
        // Shift + letter: the capital as it is (Japanese IMEs switch to Latin there).
        return Some(if p.shift {
            RomajiInput::Text(full_width(c.to_ascii_uppercase()))
        } else {
            RomajiInput::Letter(c)
        });
    }
    let c = jis_char(p.key, p.shift)?;
    Some(match (p.key, p.shift) {
        (Key::Minus, false) => RomajiInput::Text('ー'),
        (Key::Comma, false) => RomajiInput::Text('、'),
        (Key::Period, false) => RomajiInput::Text('。'),
        (Key::Slash, false) => RomajiInput::Text('・'),
        (Key::BracketRight, false) => RomajiInput::Text('「'),
        (Key::Backslash, false) => RomajiInput::Text('」'),
        _ if c == '\'' => RomajiInput::Letter('\''),
        _ => RomajiInput::Text(full_width(c)),
    })
}

pub fn romaji_takes(p: Press) -> bool {
    romaji_input(p).is_some()
}

/// Layout 21: romaji of staged presses as hiragana. A trailing n reads as ん; any other pending
/// letters stay as they are.
pub fn romaji_compose(presses: &[Press]) -> String {
    let table = romaji_table();
    let is_prefix = |s: &str| {
        table
            .iter()
            .any(|(k, _)| k.starts_with(s) && k.len() > s.len())
    };
    let lookup = |s: &str| table.iter().find(|(k, _)| *k == s).map(|(_, v)| *v);
    let mut out = String::new();
    let mut pending = String::new();
    let settle = |pending: &mut String, out: &mut String, end: bool| loop {
        if pending.is_empty() {
            return;
        }
        // "nn": ん, and a vowel after it starts the next kana on the second n (konnichiha →
        // こんにちは); a third n is that second n (konnnichiha → こんにちは too).
        if pending.starts_with("nn") {
            match pending.chars().nth(2) {
                None if !end => return,
                Some(c) if "aiueo".contains(c) => {
                    out.push('ん');
                    pending.remove(0);
                }
                _ => {
                    out.push('ん');
                    pending.drain(..2);
                    if pending.is_empty() {
                        return;
                    }
                    if pending == "n" && !end {
                        return;
                    }
                }
            }
            continue;
        }
        if let Some(kana) = lookup(pending) {
            out.push_str(kana);
            pending.clear();
            return;
        }
        if !end && is_prefix(pending) {
            return;
        }
        let first = pending.chars().next().unwrap();
        let second = pending.chars().nth(1);
        if first == 'n' && (end || second.is_some_and(|c| !"aiueoyn'".contains(c))) {
            out.push('ん');
        } else if (second == Some(first) && !"aiueon'".contains(first))
            || (first == 't' && second == Some('c'))
        {
            out.push('っ');
        } else if first != '\'' {
            out.push(first);
        }
        pending.remove(0);
    };
    for p in presses {
        match romaji_input(*p) {
            Some(RomajiInput::Letter(c)) => {
                pending.push(c);
                settle(&mut pending, &mut out, false);
            }
            Some(RomajiInput::Text(c)) => {
                settle(&mut pending, &mut out, true);
                out.push(c);
            }
            None => {}
        }
    }
    settle(&mut pending, &mut out, true);
    out
}

/// Hiragana in the katakana block (か → カ, ゔ → ヴ); everything else stays.
pub fn katakana(hiragana: &str) -> String {
    hiragana
        .chars()
        .map(|c| match c {
            'ぁ'..='ゖ' => char::from_u32(c as u32 + 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use Key::*;

    fn presses(spec: &[(Key, bool)]) -> Vec<Press> {
        spec.iter().map(|&(k, s)| Press::new(k, s)).collect()
    }
    fn romaji(text: &str) -> String {
        let keys: Vec<Press> = text
            .chars()
            .map(|c| match c {
                '-' => Press::new(Minus, false),
                ',' => Press::new(Comma, false),
                '.' => Press::new(Period, false),
                c => Press::new(
                    Key::from_code(&format!("Key{}", c.to_ascii_uppercase())),
                    false,
                ),
            })
            .collect();
        romaji_compose(&keys)
    }

    #[test]
    fn kana_layout() {
        // か゛ → が, は゜ → ぱ, ⇧つ → っ: がっこう = か゛ ⇧つ こ う
        let typed = presses(&[
            (KeyT, false),
            (BracketLeft, false),
            (KeyZ, true),
            (KeyB, false),
            (Digit4, false),
        ]);
        assert_eq!(kana_compose(&typed), "がっこう");
        assert_eq!(
            kana_compose(&presses(&[(KeyF, false), (BracketRight, false)])),
            "ぱ"
        );
        assert_eq!(
            kana_compose(&presses(&[(Digit3, false), (BracketLeft, false)])),
            "あ゛"
        );
    }

    #[test]
    fn romaji_spellings() {
        assert_eq!(romaji("konnnichiha"), "こんにちは");
        assert_eq!(romaji("konnichiha"), "こんにちは");
        assert_eq!(romaji("gakkou"), "がっこう");
        assert_eq!(romaji("shinbun"), "しんぶん");
        assert_eq!(romaji("kyouto"), "きょうと");
        assert_eq!(romaji("ra-men"), "らーめん");
        assert_eq!(romaji("kan"), "かん");
        assert_eq!(romaji("ky"), "ky");
        assert_eq!(romaji("matcha"), "まっちゃ");
        assert_eq!(romaji("arigatou."), "ありがとう。");
        assert_eq!(romaji("fairu"), "ふぁいる");
        assert_eq!(romaji("konn"), "こん");
        assert_eq!(romaji("annna"), "あんな");
        assert_eq!(romaji("kinnyoubi"), "きんようび");
    }

    #[test]
    fn scripts() {
        assert_eq!(katakana("らーめん"), "ラーメン");
    }
}
