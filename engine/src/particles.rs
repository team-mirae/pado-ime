//! Particles and endings that follow a word of another language in one staged run (docs/spec.md
//! §7.0, "A word and its particle"): Claude에, report를, client들에게; Claudeの, GitHubで. The lists
//! are JIS Moonshot's (`engine/src/predict.rs` there; moonshot.md §5.3, §5.5).

/// Korean particles and endings that attach to an English word in mixed Korean text.
const KOREAN: &[&str] = &[
    "은", "는", "이", "가", "을", "를", "의", "에", "에서", "에게", "에는", "에서는", "에서의", "에도",
    "과", "와", "도", "만", "로", "으로", "로는", "으로는", "로도", "으로도", "까지", "부터", "까지는", "까지의", "부터는", "부터의", "부로", "처럼", "보다",
    "이나", "나", "랑", "이랑", "한테", "께서", "이다", "입니다", "이고", "이며", "이라고", "라고", "이라는",
    "라는", "인", "적", "적인", "적으로", "들", "들이", "들은", "들을", "들의", "하는", "한", "하고", "했다",
    "하다", "해서", "하면", "된", "되는", "화", "화된", "용", "형", "급", "씨", "님",
    // Business text: FDA로부터, Dynamics와의, Licensee로서, CRO이자, Innovation이란.
    "로부터", "으로부터", "로부터의", "로서", "으로서", "로써", "으로써", "로의", "으로의", "와의", "과의", "와는",
    "과는", "와도", "과도", "와를", "과를", "에서와", "에게서", "에서도", "만의", "이자", "이란", "란", "이라", "라",
    "등", "등의", "등을", "등이", "등과", "등은", "였다", "이었다", "였으며", "이었으며", "이지만", "임", "임을",
];

/// Korean particles that can come first in a stack (들 + 에게, 부터 + 가); 이 가 을 를 은 는 의 end one.
const FIRST: &[&str] = &["들", "에", "에서", "에게", "한테", "께", "부터", "까지", "로", "으로", "만", "처럼", "보다", "과", "와", "랑", "이랑", "하고", "께서"];

/// Japanese particles and endings that follow a name (Claudeの, GitHubで).
const JAPANESE: &[&str] = &[
    "は", "が", "を", "に", "で", "と", "も", "へ", "の", "や", "か", "ね", "よ", "から", "まで", "より",
    "には", "では", "とは", "にも", "でも", "への", "との", "での", "からの", "までの", "って", "けど",
    "だ", "です", "でした", "という", "など", "さん", "くん", "ちゃん", "さま",
];

/// Whether `text` is a Korean particle or ending, or two or three stacked (client들에게, naming부터가).
pub fn korean(text: &str) -> bool {
    if KOREAN.contains(&text) {
        return true;
    }
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    // parts[i]: the fewest particles the first i characters are, if they are particles.
    let mut parts = vec![usize::MAX; n + 1];
    parts[0] = 0;
    for end in 1..=n {
        for start in 0..end {
            let piece: String = chars[start..end].iter().collect();
            let fits = KOREAN.contains(&piece.as_str()) && (end == n || FIRST.contains(&piece.as_str()));
            if parts[start] < 3 && fits {
                parts[end] = parts[end].min(parts[start] + 1);
            }
        }
    }
    (2..=3).contains(&parts[n])
}

/// Whether `text` is a Japanese particle or ending that follows a name.
pub fn japanese(text: &str) -> bool {
    JAPANESE.contains(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particles_and_stacks() {
        assert!(korean("에") && korean("에서는") && korean("들에게") && korean("부터가") && korean("들에게는"));
        assert!(!korean("이의") && !korean("사과") && !korean("") && !korean("에에에에"));
        assert!(japanese("の") && japanese("からの") && !japanese("ねこ"));
    }
}
