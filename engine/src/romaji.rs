//! Kana → canonical romaji, for the menu's notes (docs/spec.md §11.3, "Menu notes"): Hepburn as
//! Japanese IMEs spell it (shi, chi, tsu, fu, ji, kya, sha), っ doubling the next consonant (tch
//! before ch), ん as n (n' before a vowel or y), ー as -. Katakana reads as hiragana. Anything else
//! (kanji, Latin, punctuation) passes through, 、。 as , and . .

use crate::japanese::DAKUTEN;

fn mora(c: char) -> Option<&'static str> {
    Some(match c {
        'あ' => "a", 'い' => "i", 'う' => "u", 'え' => "e", 'お' => "o",
        'か' => "ka", 'き' => "ki", 'く' => "ku", 'け' => "ke", 'こ' => "ko",
        'が' => "ga", 'ぎ' => "gi", 'ぐ' => "gu", 'げ' => "ge", 'ご' => "go",
        'さ' => "sa", 'し' => "shi", 'す' => "su", 'せ' => "se", 'そ' => "so",
        'ざ' => "za", 'じ' => "ji", 'ず' => "zu", 'ぜ' => "ze", 'ぞ' => "zo",
        'た' => "ta", 'ち' => "chi", 'つ' => "tsu", 'て' => "te", 'と' => "to",
        'だ' => "da", 'ぢ' => "ji", 'づ' => "zu", 'で' => "de", 'ど' => "do",
        'な' => "na", 'に' => "ni", 'ぬ' => "nu", 'ね' => "ne", 'の' => "no",
        'は' => "ha", 'ひ' => "hi", 'ふ' => "fu", 'へ' => "he", 'ほ' => "ho",
        'ば' => "ba", 'び' => "bi", 'ぶ' => "bu", 'べ' => "be", 'ぼ' => "bo",
        'ぱ' => "pa", 'ぴ' => "pi", 'ぷ' => "pu", 'ぺ' => "pe", 'ぽ' => "po",
        'ま' => "ma", 'み' => "mi", 'む' => "mu", 'め' => "me", 'も' => "mo",
        'や' => "ya", 'ゆ' => "yu", 'よ' => "yo",
        'ら' => "ra", 'り' => "ri", 'る' => "ru", 'れ' => "re", 'ろ' => "ro",
        'わ' => "wa", 'ゐ' => "wi", 'ゑ' => "we", 'を' => "wo", 'ゔ' => "vu",
        'ぁ' => "xa", 'ぃ' => "xi", 'ぅ' => "xu", 'ぇ' => "xe", 'ぉ' => "xo",
        'ゃ' => "xya", 'ゅ' => "xyu", 'ょ' => "xyo", 'ゎ' => "xwa",
        'ー' => "-", '、' => ",", '。' => ".", '・' => "/", '「' => "[", '」' => "]",
        _ => return None,
    })
}

/// A kana followed by a small kana, read as one (きゃ kya, しゃ sha, ふぁ fa, てぃ ti, ゔぁ va).
fn digraph(a: char, b: char) -> Option<String> {
    let small = match b {
        'ゃ' => "a", 'ゅ' => "u", 'ょ' => "o",
        'ぁ' => "a", 'ぃ' => "i", 'ぅ' => "u", 'ぇ' => "e", 'ぉ' => "o",
        _ => return None,
    };
    let y_row = matches!(b, 'ゃ' | 'ゅ' | 'ょ');
    let stem = match a {
        'し' if y_row => "sh", 'ち' if y_row => "ch", 'じ' | 'ぢ' if y_row => "j",
        'き' | 'ぎ' | 'に' | 'ひ' | 'び' | 'ぴ' | 'み' | 'り' if y_row => {
            return Some(format!("{}y{small}", &mora(a)?[..1]));
        }
        'し' => "sh", 'ち' => "ch", 'じ' => "j",
        'ふ' => "f", 'ゔ' => "v", 'て' => "t", 'で' => "d", 'う' => "w", 'つ' => "ts",
        _ => return None,
    };
    Some(format!("{stem}{small}"))
}

fn to_hiragana(c: char) -> char {
    match c {
        'ァ'..='ヶ' => char::from_u32(c as u32 - 0x60).unwrap_or(c),
        _ => c,
    }
}

/// Canonical romaji of kana text; None if it holds no kana at all.
pub fn romaji(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().map(to_hiragana).collect();
    if !chars.iter().any(|&c| mora(c).is_some() || c == 'っ' || c == 'ん') {
        return None;
    }
    let mut out = String::new();
    let mut i = 0;
    let mut sokuon = false;
    while i < chars.len() {
        let c = chars[i];
        if c == 'っ' {
            sokuon = true;
            i += 1;
            continue;
        }
        if c == 'ん' {
            let next = chars.get(i + 1).and_then(|&n| mora(n)).unwrap_or("");
            out.push('n');
            if next.starts_with(['a', 'i', 'u', 'e', 'o', 'y']) {
                out.push('\'');
            }
            i += 1;
            continue;
        }
        let (syl, step) = match chars.get(i + 1).and_then(|&b| digraph(c, b)) {
            Some(d) => (d, 2),
            None => match mora(c) {
                Some(m) => (m.to_string(), 1),
                None if c == DAKUTEN => ("\"".to_string(), 1),
                None => (c.to_string(), 1),
            },
        };
        if sokuon {
            if syl.starts_with("ch") {
                out.push('t');
            } else if let Some(f) = syl.chars().next().filter(|f| f.is_ascii_alphabetic() && !"aiueo".contains(*f)) {
                out.push(f);
            } else {
                out.push_str("xtu");
            }
            sokuon = false;
        }
        out.push_str(&syl);
        i += step;
    }
    if sokuon {
        out.push_str("xtu");
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hepburn() {
        assert_eq!(romaji("こんにちは").unwrap(), "konnichiha");
        assert_eq!(romaji("がっこう").unwrap(), "gakkou");
        assert_eq!(romaji("まっちゃ").unwrap(), "matcha");
        assert_eq!(romaji("きょうと").unwrap(), "kyouto");
        assert_eq!(romaji("ラーメン").unwrap(), "ra-men");
        assert_eq!(romaji("しんよう").unwrap(), "shin'you");
        assert_eq!(romaji("ふぁいる").unwrap(), "fairu");
        assert_eq!(romaji("ありがとう。").unwrap(), "arigatou.");
        assert_eq!(romaji("じゃあ").unwrap(), "jaa");
        assert!(romaji("漢字").is_none());
    }
}
