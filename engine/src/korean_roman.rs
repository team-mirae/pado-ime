//! Layout 18, JIS Korean Roman (docs/layouts/18-korean-roman.md): Korean typed as it's romanized (Revised
//! Romanization and common variants: annyeonghaseyo → 안녕하세요). Like Korean (Kana) (19), the
//! default reading shows and every other reading of the letters is kept: spellings that stand for
//! several jamo (k ㅋ/ㄱ, o ㅗ/ㅓ, t as a final ㅅ/ㄷ/ㅆ …) and other syllable splits.
//!
//! The default reading: each consonant run between vowels gives the next syllable the longest
//! onset it can take, the rest is the final (hanguk → 한국, jeongmal → 정말); the first spelling of
//! each table entry; a hyphen splits syllables (han-guk).

use crate::hangul::compose;
use crate::layout::{Press, jis_char};

/// Onsets, longest first: (spelling, jamo, default first).
const ONSETS: &[(&str, &[char])] = &[
    ("kk", &['ㄲ']), ("gg", &['ㄲ']), ("tt", &['ㄸ']), ("dd", &['ㄸ']), ("pp", &['ㅃ']), ("bb", &['ㅃ']),
    ("ss", &['ㅆ']), ("jj", &['ㅉ']), ("ch", &['ㅊ', 'ㅈ']), ("sh", &['ㅅ']),
    ("k", &['ㅋ', 'ㄱ']), ("g", &['ㄱ']), ("n", &['ㄴ']), ("t", &['ㅌ', 'ㄷ']), ("d", &['ㄷ']),
    ("r", &['ㄹ']), ("l", &['ㄹ']), ("m", &['ㅁ']), ("p", &['ㅍ', 'ㅂ']), ("b", &['ㅂ']),
    ("s", &['ㅅ', 'ㅆ']), ("j", &['ㅈ']), ("c", &['ㅊ']), ("h", &['ㅎ']), ("f", &['ㅍ']), ("v", &['ㅂ']), ("z", &['ㅈ']),
];

/// Vowels, longest first.
const VOWELS: &[(&str, &[char])] = &[
    ("yae", &['ㅒ', 'ㅖ']), ("wae", &['ㅙ', 'ㅞ', 'ㅚ']), ("yeo", &['ㅕ']), ("weo", &['ㅝ']),
    ("ae", &['ㅐ', 'ㅔ']), ("ya", &['ㅑ']), ("eo", &['ㅓ']), ("ye", &['ㅖ', 'ㅒ']), ("wa", &['ㅘ']),
    ("oe", &['ㅚ', 'ㅙ', 'ㅞ']), ("yo", &['ㅛ', 'ㅕ']), ("wo", &['ㅝ']), ("we", &['ㅞ', 'ㅙ', 'ㅚ']),
    ("wi", &['ㅟ']), ("yu", &['ㅠ']), ("eu", &['ㅡ']), ("ui", &['ㅢ']), ("oo", &['ㅜ']), ("ee", &['ㅣ']),
    ("a", &['ㅏ']), ("e", &['ㅔ', 'ㅐ']), ("o", &['ㅗ', 'ㅓ']), ("u", &['ㅜ', 'ㅡ']), ("i", &['ㅣ']),
];

/// Finals. Pronounced spellings stand for several (t: ㅅ ㄷ ㅆ ㅈ ㅊ ㅌ ㅎ; m before n: ㅂ, 합니다).
const CODAS: &[(&str, &[char])] = &[
    ("ng", &['ㅇ', 'ㄱ']), ("kk", &['ㄲ']), ("ss", &['ㅆ']),
    ("lk", &['ㄺ']), ("lg", &['ㄺ']), ("lm", &['ㄻ']), ("lb", &['ㄼ']), ("lp", &['ㄼ']), ("lh", &['ㅀ']),
    ("ps", &['ㅄ']), ("bs", &['ㅄ']), ("nh", &['ㄶ']), ("nj", &['ㄵ']), ("ks", &['ㄳ']),
    ("k", &['ㄱ', 'ㅋ', 'ㄲ']), ("g", &['ㄱ']), ("n", &['ㄴ']), ("t", &['ㅅ', 'ㄷ', 'ㅆ', 'ㅈ', 'ㅊ', 'ㅌ', 'ㅎ']),
    ("d", &['ㄷ']), ("l", &['ㄹ']), ("r", &['ㄹ']), ("m", &['ㅁ', 'ㅂ']), ("p", &['ㅂ', 'ㅍ']), ("b", &['ㅂ']),
    ("s", &['ㅅ']),
];

fn is_vowel_letter(c: u8) -> bool {
    b"aeiouwy".contains(&c)
}

fn matches<'a>(table: &'a [(&str, &[char])], s: &[u8]) -> Vec<(usize, &'a [char])> {
    table.iter().filter(|(sp, _)| s.starts_with(sp.as_bytes())).map(|(sp, j)| (sp.len(), *j)).collect()
}


/// What the presses spell: runs of letters, hyphens (a syllable break) and other characters.
#[derive(Debug, PartialEq, Eq)]
enum Piece {
    Letters(Vec<u8>),
    Break,
    Lit(char),
}

fn pieces(presses: &[Press]) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    for p in presses {
        let Some(c) = jis_char(p.key, p.shift) else { continue };
        if p.key.is_letter() {
            let c = c.to_ascii_lowercase() as u8;
            match out.last_mut() {
                Some(Piece::Letters(run)) => run.push(c),
                _ => out.push(Piece::Letters(vec![c])),
            }
        } else if c == '-' || c == '\'' {
            out.push(Piece::Break);
        } else {
            out.push(Piece::Lit(c));
        }
    }
    out
}

pub fn takes(p: Press) -> bool {
    !p.alt && jis_char(p.key, p.shift).is_some()
}

/// One syllable as parsed: its jamo options and where it ends.
struct Parsed {
    onset: &'static [char],
    vowel: &'static [char],
    coda: &'static [char],
    end: usize,
}

/// Every way a syllable can start at `i` in `run`, the default first.
fn syllables_at(run: &[u8], i: usize) -> Vec<Parsed> {
    const NONE: &[char] = &[];
    const IEUNG: &[char] = &['ㅇ'];
    let mut out = Vec::new();
    let mut onsets: Vec<(usize, &'static [char])> = matches(ONSETS, &run[i..]).into_iter().map(|(l, j)| (l, j)).collect();
    if run.get(i).is_some_and(|&c| is_vowel_letter(c)) {
        onsets.push((0, IEUNG));
    }
    for (ol, onset) in onsets {
        let j = i + ol;
        for (vl, vowel) in matches(VOWELS, &run[j..]) {
            let k = j + vl;
            // The consonant run after the vowel, up to the next vowel letter or the end.
            let run_end = (k..run.len()).find(|&x| is_vowel_letter(run[x])).unwrap_or(run.len());
            let cons = &run[k..run_end];
            // Every final that fits at the start of the run, the default first: at the end the
            // whole run; before a vowel, the shortest final that leaves the next syllable the
            // longest onset (none if the whole run is an onset).
            let mut codas: Vec<(usize, &'static [char])> = matches(CODAS, cons).into_iter().map(|(l, c)| (l, c)).collect();
            codas.push((0, NONE));
            let at_end = run_end == run.len();
            let preferred = |len: usize| -> bool {
                let rest = &cons[len..];
                if at_end {
                    rest.is_empty()
                } else {
                    rest.is_empty() || ONSETS.iter().any(|(sp, _)| sp.as_bytes() == rest)
                }
            };
            codas.sort_by_key(|&(len, _)| {
                let rank = if preferred(len) { 0 } else { 1 };
                // Among preferred ones, the shortest final (the longest onset after it).
                (rank, if rank == 0 { len } else { usize::MAX - len })
            });
            for (cl, coda) in codas {
                out.push(Parsed { onset, vowel, coda, end: k + cl });
            }
        }
    }
    out
}

/// Readings of the presses, the default first, at most `limit`.
pub fn readings(presses: &[Press], limit: usize) -> Vec<String> {
    let parts = pieces(presses);
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut budget = 200_000usize;
    walk(&parts, 0, 0, &mut String::new(), &mut out, &mut seen, limit, &mut budget);
    if out.is_empty() {
        // Nothing parses (a lone consonant, a pending y): the letters as they are, jamo where they
        // spell one.
        out.push(loose(&parts));
    }
    out
}

pub fn compose_presses(presses: &[Press]) -> String {
    readings(presses, 1).into_iter().next().unwrap_or_default()
}

fn loose(parts: &[Piece]) -> String {
    let mut s = String::new();
    for p in parts {
        match p {
            Piece::Letters(run) => {
                let mut i = 0;
                // Whole syllables while they parse, then the rest letter by letter.
                while i < run.len() {
                    if let Some(p) = syllables_at(run, i).into_iter().next() {
                        s.push(compose(p.onset[0], p.vowel[0], p.coda.first().copied()).unwrap_or('?'));
                        i = p.end;
                        continue;
                    }
                    let one = matches(ONSETS, &run[i..i + 1]);
                    match one.first() {
                        Some((_, j)) => s.push(j[0]),
                        None => s.push(run[i] as char),
                    }
                    i += 1;
                }
            }
            Piece::Break => {}
            Piece::Lit(c) => s.push(*c),
        }
    }
    s
}

#[allow(clippy::too_many_arguments)]
fn walk(parts: &[Piece], pi: usize, i: usize, built: &mut String, out: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>, limit: usize, budget: &mut usize) {
    if out.len() >= limit || *budget == 0 {
        return;
    }
    *budget -= 1;
    let Some(part) = parts.get(pi) else {
        if seen.insert(built.clone()) {
            out.push(built.clone());
        }
        return;
    };
    match part {
        Piece::Break => walk(parts, pi + 1, 0, built, out, seen, limit, budget),
        Piece::Lit(c) => {
            built.push(*c);
            walk(parts, pi + 1, 0, built, out, seen, limit, budget);
            built.pop();
        }
        Piece::Letters(run) => {
            if i == run.len() {
                return walk(parts, pi + 1, 0, built, out, seen, limit, budget);
            }
            for p in syllables_at(run, i) {
                let codas: Vec<Option<char>> = if p.coda.is_empty() { vec![None] } else { p.coda.iter().map(|&c| Some(c)).collect() };
                for &l in p.onset {
                    for &v in p.vowel {
                        for &c in &codas {
                            let Some(s) = compose(l, v, c) else { continue };
                            built.push(s);
                            walk(parts, pi, p.end, built, out, seen, limit, budget);
                            built.pop();
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;

    fn presses(text: &str) -> Vec<Press> {
        text.chars()
            .map(|c| match c {
                '-' => Press::new(Key::Minus, false),
                ',' => Press::new(Key::Comma, false),
                c => Press::new(Key::from_code(&format!("Key{}", c.to_ascii_uppercase())), false),
            })
            .collect()
    }
    fn default(text: &str) -> String {
        compose_presses(&presses(text))
    }
    fn all(text: &str) -> Vec<String> {
        readings(&presses(text), 100_000)
    }

    #[test]
    fn defaults() {
        assert_eq!(default("annyeonghaseyo"), "안녕하세요");
        assert_eq!(default("hanguk"), "한국");
        assert_eq!(default("jeongmal"), "정말");
        assert_eq!(default("saranghae"), "사랑해");
        assert_eq!(default("kkachi"), "까치");
        assert_eq!(default("seoul"), "서울");
        assert_eq!(default("han-guk"), "한국");
        assert_eq!(default("hang-uk"), "항욱");
        assert_eq!(default("an"), "안");
        assert_eq!(default("ann"), "안ㄴ");
        assert_eq!(default("anny"), "안ㄴy");
    }

    #[test]
    fn alternatives() {
        // m before n can be ㅂ: gamsahamnida → 감사합니다
        assert!(all("gamsahamnida").contains(&"감사합니다".to_string()));
        // k ㄱ, o ㅓ: kamsa → 감사; meokda? t final ㅅ: mat → 맛
        assert!(all("kamsa").contains(&"감사".to_string()));
        assert!(all("mat").contains(&"맛".to_string()));
        assert!(all("cheonmaneyo").contains(&"천만에요".to_string()));
        // another split: hanguk → 항욱 too
        assert!(all("hanguk").contains(&"항욱".to_string()));
    }
}
