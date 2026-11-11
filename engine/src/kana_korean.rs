//! Layout 19, JIS Kana Korean (docs/layouts/19-kana-korean.md): Korean typed on the JIS kana layout, each kana
//! key its Korean approximation (た → 타, し → 시). ゛ ゜ voice the previous syllable's initial,
//! small vowels (⇧ 3–0) change its vowel, ん / ⇧ん / ⇧む add a final, and ⇧Z (っ) makes the next
//! initial tense. Layout from docs/layouts/19-kana-korean.json.

use crate::hangul::{compose, compound, decompose};
use crate::key::Key;
use crate::layout::Press;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unit {
    /// A whole syllable (initial, vowel).
    Syl(char, char),
    /// A vowel on the implied ㅇ, or joining a lone initial (い).
    Vowel(char),
    /// A small vowel: changes the previous syllable's vowel (き + ゃ → 캬).
    Small(char),
    /// A final for the previous syllable (ん ㄴ, ⇧ん ㅇ, ⇧む ㅁ).
    Final(char),
    /// A consonant, final or initial as it falls (つ: ㅋ).
    Cons(char),
    Dakuten,
    Handakuten,
    /// っ: the next initial goes tense.
    Tense,
    Lit(char),
}

fn unit(key: Key, shift: bool) -> Option<Unit> {
    use Key::*;
    use Unit::*;
    let s = |text: &str| {
        let (l, v, _) = decompose(text.chars().next().unwrap()).unwrap();
        Syl(l, v)
    };
    if shift {
        let small = match key {
            Digit3 => Some('ㅏ'),
            Digit4 => Some('ㅜ'),
            Digit5 => Some('ㅔ'),
            Digit6 => Some('ㅗ'),
            Digit7 => Some('ㅑ'),
            Digit8 => Some('ㅠ'),
            Digit9 => Some('ㅛ'),
            Digit0 => Some('ㅘ'),
            _ => None,
        };
        if let Some(v) = small {
            return Some(Small(v));
        }
        match key {
            KeyY => return Some(Final('ㅇ')),
            Backslash => return Some(Final('ㅁ')),
            KeyZ => return Some(Tense),
            // の taken literally: the particle 의, completed (nothing attaches to it).
            KeyK => return Some(Lit('의')),
            _ => {}
        }
    }
    Some(match key {
        Digit1 => s("누"),
        Digit2 => s("후"),
        Digit3 => s("아"),
        Digit4 => s("우"),
        Digit5 => s("에"),
        Digit6 => s("오"),
        Digit7 => s("야"),
        Digit8 => s("유"),
        Digit9 => s("요"),
        Digit0 => s("와"),
        Minus => s("호"),
        Equal => s("헤"),
        IntlYen => Lit('ー'),
        KeyQ => s("타"),
        KeyW => s("테"),
        KeyE => Vowel('ㅣ'),
        KeyR => s("스"),
        KeyT => s("카"),
        KeyY => Final('ㄴ'),
        KeyU => s("나"),
        KeyI => s("니"),
        KeyO => s("라"),
        KeyP => s("세"),
        BracketLeft => Dakuten,
        BracketRight => Handakuten,
        KeyA => s("치"),
        KeyS => s("토"),
        KeyD => s("시"),
        KeyF => s("하"),
        KeyG => s("키"),
        KeyH => s("쿠"),
        KeyJ => s("마"),
        KeyK => s("노"),
        KeyL => s("리"),
        Semicolon => s("레"),
        Quote => s("케"),
        Backslash => s("무"),
        KeyZ => Cons('ㅋ'),
        KeyX => s("사"),
        KeyC => s("소"),
        KeyV => s("히"),
        KeyB => s("코"),
        KeyN => s("미"),
        KeyM => s("모"),
        Comma => s("네"),
        Period => s("루"),
        Slash => s("메"),
        IntlRo => s("로"),
        _ => return None,
    })
}

pub fn takes(p: Press) -> bool {
    !p.alt && unit(p.key, p.shift).is_some()
}

/// ゛: か → が (ㅋ → ㄱ), た → だ (ㅌ → ㄷ), さ → ざ (ㅅ → ㅈ), ち → ぢ (ㅊ → ㅈ), は → ば (ㅎ → ㅂ), う → ゔ (ㅇ → ㅂ).
fn voiced(l: char) -> Option<char> {
    Some(match l {
        'ㅋ' => 'ㄱ',
        'ㅌ' => 'ㄷ',
        'ㅅ' => 'ㅈ',
        'ㅊ' => 'ㅈ',
        'ㅎ' => 'ㅂ',
        'ㅇ' => 'ㅂ',
        _ => return None,
    })
}

/// ゜: は → ぱ (ㅎ → ㅍ).
fn semi_voiced(l: char) -> Option<char> {
    (l == 'ㅎ').then_some('ㅍ')
}

/// っ: the next initial one step tenser (ㄱ/ㅋ → ㄲ, ㄷ/ㅌ → ㄸ, ㅂ/ㅍ → ㅃ, ㅅ → ㅆ, ㅈ/ㅊ → ㅉ).
fn tense(l: char) -> char {
    match l {
        'ㄱ' | 'ㅋ' => 'ㄲ',
        'ㄷ' | 'ㅌ' => 'ㄸ',
        'ㅂ' | 'ㅍ' => 'ㅃ',
        'ㅅ' => 'ㅆ',
        'ㅈ' | 'ㅊ' => 'ㅉ',
        other => other,
    }
}

#[derive(Clone, Debug, Default)]
struct Syl {
    ini: Option<char>,
    vowel: Option<char>,
    fin: Option<char>,
    /// Made tense by a っ before it: a ゛ / ゜ after it re-applies the tense step.
    tensed: bool,
    /// The initial before the っ made it tense (for the reading where っ is a final ㅅ instead).
    plain_ini: Option<char>,
    /// The final came from plain ん (ㄴ or ㅇ).
    fin_from_n: bool,
    /// The initial came from plain ん before a small vowel (ㄴ or ㅇ).
    ini_from_n: bool,
    lit: Option<char>,
}

/// The default reading: what the table says (docs/layouts/19-kana-korean.md).
pub fn compose_presses(presses: &[Press]) -> String {
    render(&syllables(presses))
}

fn render(syls: &[Syl]) -> String {
    let mut out = String::new();
    for s in syls {
        match (s.lit, s.ini, s.vowel) {
            (Some(c), _, _) => out.push(c),
            (None, Some(l), Some(v)) => out.push(compose(l, v, s.fin).unwrap_or(l)),
            (None, Some(l), None) => out.push(l),
            _ => {}
        }
    }
    out
}

/// The vowels a kana's vowel may stand for (docs/layouts/19-kana-korean.md, "Readings"): う-row ㅜ ㅡ,
/// お-row ㅗ ㅓ, ょ ㅛ ㅕ.
fn vowel_options(v: char) -> &'static [char] {
    match v {
        'ㅜ' => &['ㅜ', 'ㅡ'],
        'ㅗ' => &['ㅗ', 'ㅓ'],
        'ㅛ' => &['ㅛ', 'ㅕ'],
        'ㅏ' => &['ㅏ'],
        'ㅣ' => &['ㅣ'],
        'ㅔ' => &['ㅔ'],
        'ㅑ' => &['ㅑ'],
        'ㅠ' => &['ㅠ'],
        'ㅘ' => &['ㅘ'],
        'ㅞ' => &['ㅞ'],
        'ㅐ' => &['ㅐ'],
        'ㅚ' => &['ㅚ'],
        'ㅟ' => &['ㅟ'],
        'ㅢ' => &['ㅢ'],
        'ㅓ' => &['ㅓ'],
        'ㅕ' => &['ㅕ'],
        'ㅡ' => &['ㅡ'],
        'ㅒ' => &['ㅒ'],
        'ㅖ' => &['ㅖ'],
        'ㅙ' => &['ㅙ'],
        'ㅝ' => &['ㅝ'],
        _ => &[],
    }
}

/// The final a う-row kana spells in Japanese renderings of Korean (ク ㄱ, ム ㅁ, ル ㄹ, プ ㅂ).
fn as_final(s: &Syl) -> Option<char> {
    if s.lit.is_some() || s.fin.is_some() || s.vowel != Some('ㅜ') || s.tensed {
        return None;
    }
    Some(match s.ini? {
        'ㅋ' | 'ㄱ' => 'ㄱ',
        'ㅁ' => 'ㅁ',
        'ㄹ' => 'ㄹ',
        'ㅍ' | 'ㅂ' => 'ㅂ',
        _ => return None,
    })
}

/// Every Korean reading of the presses (docs/layouts/19-kana-korean.md, "Readings"), the default first, at
/// most `limit` (searched depth-first, the default choice first at each kana).
pub fn readings(presses: &[Press], limit: usize) -> Vec<String> {
    let syls = syllables(presses);
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut built: Vec<Syl> = Vec::new();
    expand(&syls, 0, &mut built, &mut out, &mut seen, limit);
    out
}

fn expand(
    syls: &[Syl],
    i: usize,
    built: &mut Vec<Syl>,
    out: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
    limit: usize,
) {
    if out.len() >= limit {
        return;
    }
    let Some(s) = syls.get(i) else {
        let text = render(built);
        if seen.insert(text.clone()) {
            out.push(text);
        }
        return;
    };
    let prev_open = built
        .last()
        .is_some_and(|p| p.lit.is_none() && p.vowel.is_some() && p.fin.is_none());
    // The syllable as itself, each of its vowels and finals.
    let vowels: Vec<Option<char>> = match s.vowel {
        Some(v) if !vowel_options(v).is_empty() => {
            vowel_options(v).iter().map(|&c| Some(c)).collect()
        }
        other => vec![other],
    };
    let fins: Vec<Option<char>> = if s.fin_from_n {
        vec![Some('ㄴ'), Some('ㅇ')]
    } else {
        vec![s.fin]
    };
    let inis: Vec<Option<char>> = if s.ini_from_n {
        vec![Some('ㄴ'), Some('ㅇ')]
    } else {
        vec![s.ini]
    };
    // っ: the tense initial (default), or a ㅅ final on the syllable before and the plain initial.
    let sokuon = if s.tensed && prev_open {
        vec![false, true]
    } else {
        vec![false]
    };
    for &as_s in &sokuon {
        for ini in &inis {
            for v in &vowels {
                for f in &fins {
                    let mut t = s.clone();
                    t.ini = *ini;
                    t.vowel = *v;
                    t.fin = *f;
                    if as_s {
                        t.ini = t.plain_ini;
                        built.last_mut().unwrap().fin = Some('ㅅ');
                    }
                    built.push(t);
                    expand(syls, i + 1, built, out, seen, limit);
                    built.pop();
                    if as_s {
                        built.last_mut().unwrap().fin = None;
                    }
                }
            }
        }
    }
    // ング: a ぐ / く after an ㅇ final spells that ㅇ (アンニョング → 안녕), and drops out.
    let after_ng = built
        .last()
        .is_some_and(|p| p.lit.is_none() && p.fin == Some('ㅇ'));
    if after_ng
        && s.lit.is_none()
        && s.fin.is_none()
        && !s.tensed
        && s.vowel == Some('ㅜ')
        && matches!(s.ini, Some('ㄱ' | 'ㅋ'))
    {
        expand(syls, i + 1, built, out, seen, limit);
    }
    // A う-row kana as the final of the syllable before (한국 ハングク).
    if let (true, Some(f)) = (prev_open, as_final(s)) {
        built.last_mut().unwrap().fin = Some(f);
        expand(syls, i + 1, built, out, seen, limit);
        built.last_mut().unwrap().fin = None;
    }
}

fn syllables(presses: &[Press]) -> Vec<Syl> {
    let mut syls: Vec<Syl> = Vec::new();
    let mut pending_tense = false;
    let start = |syls: &mut Vec<Syl>, ini: char, vowel: Option<char>, pending: &mut bool| {
        let tensed = std::mem::take(pending);
        let plain = ini;
        let ini = if tensed { tense(ini) } else { ini };
        syls.push(Syl {
            ini: Some(ini),
            vowel,
            tensed,
            plain_ini: Some(plain),
            ..Syl::default()
        });
    };
    // The last press was ん: Some(true) for ⇧ん (ㅇ), Some(false) for plain ん (ㄴ or ㅇ).
    let mut last_n: Option<bool> = None;
    for p in presses {
        let Some(u) = unit(p.key, p.shift) else {
            continue;
        };
        let n_before = last_n.take();
        if p.key == Key::KeyY {
            last_n = Some(p.shift);
        }
        let open = |s: &Syl| s.lit.is_none() && s.vowel.is_some() && s.fin.is_none();
        match u {
            Unit::Syl(l, v) => start(&mut syls, l, Some(v), &mut pending_tense),
            Unit::Vowel(v) => match syls.last_mut() {
                Some(s) if s.lit.is_none() && s.ini.is_some() && s.vowel.is_none() => {
                    s.vowel = Some(v)
                }
                // After a final from a consonant key, the vowel takes it as its initial.
                Some(s) if s.lit.is_none() && s.vowel.is_some() && s.fin == Some('ㅋ') => {
                    s.fin = None;
                    start(&mut syls, 'ㅋ', Some(v), &mut pending_tense);
                }
                _ => start(&mut syls, 'ㅇ', Some(v), &mut pending_tense),
            },
            // ん then a small vowel: a syllable on ㄴ or ㅇ (plain ん), or ㅇ (⇧ん), not a final.
            Unit::Small(v) if n_before.is_some() => {
                match syls.last_mut() {
                    Some(s) if s.lit.is_some() => {
                        syls.pop();
                    }
                    Some(s) => {
                        s.fin = None;
                        s.fin_from_n = false;
                    }
                    None => {}
                }
                let plain = n_before == Some(false);
                let ini = if plain { 'ㄴ' } else { 'ㅇ' };
                syls.push(Syl {
                    ini: Some(ini),
                    vowel: Some(v),
                    plain_ini: Some(ini),
                    ini_from_n: plain,
                    ..Syl::default()
                });
            }
            // っ then a small vowel: ㅋ with that vowel (⇧Z ⇧ぁ → 카).
            Unit::Small(v) if pending_tense => {
                pending_tense = false;
                syls.push(Syl {
                    ini: Some('ㅋ'),
                    vowel: Some(v),
                    plain_ini: Some('ㅋ'),
                    ..Syl::default()
                });
            }
            Unit::Small(v) => match syls.last_mut() {
                Some(s) if s.lit.is_none() && s.vowel.is_some() && s.fin.is_none() => {
                    let old = s.vowel.unwrap();
                    s.vowel = Some(compound(old, v).unwrap_or(v));
                }
                _ => start(&mut syls, 'ㅇ', Some(v), &mut pending_tense),
            },
            Unit::Final(f) => match syls.last_mut() {
                Some(s) if open(s) => {
                    s.fin = Some(f);
                    s.fin_from_n = f == 'ㄴ' && p.key == Key::KeyY && !p.shift;
                }
                _ => syls.push(Syl {
                    lit: Some(f),
                    ..Syl::default()
                }),
            },
            Unit::Cons(c) => match syls.last_mut() {
                Some(s) if open(s) && !pending_tense => s.fin = Some(c),
                _ => start(&mut syls, c, None, &mut pending_tense),
            },
            Unit::Dakuten | Unit::Handakuten => {
                if let Some(s) = syls
                    .last_mut()
                    .filter(|s| s.lit.is_none() && s.fin.is_none())
                {
                    if let Some(l) = s.ini {
                        let base = if u == Unit::Dakuten {
                            voiced(l)
                        } else {
                            semi_voiced(l)
                        };
                        if let Some(b) = base {
                            s.ini = Some(if s.tensed { tense(b) } else { b });
                            s.plain_ini = Some(b);
                        }
                    }
                }
            }
            Unit::Tense => pending_tense = true,
            Unit::Lit('ー') => match syls.last_mut() {
                // ー after a short ㅔ / ㅖ: its long form ㅐ / ㅒ, and no ー.
                Some(s)
                    if s.lit.is_none()
                        && s.fin.is_none()
                        && matches!(s.vowel, Some('ㅔ' | 'ㅖ')) =>
                {
                    s.vowel = s.vowel.map(|v| if v == 'ㅔ' { 'ㅐ' } else { 'ㅒ' });
                }
                _ => syls.push(Syl {
                    lit: Some('ー'),
                    ..Syl::default()
                }),
            },
            Unit::Lit(c) => syls.push(Syl {
                lit: Some(c),
                ..Syl::default()
            }),
        }
    }
    syls
}

#[cfg(test)]
mod tests {
    use super::*;
    use Key::*;

    fn typed(keys: &[(Key, bool)]) -> String {
        compose_presses(
            &keys
                .iter()
                .map(|&(k, s)| Press::new(k, s))
                .collect::<Vec<_>>(),
        )
    }
    fn plain(keys: &[Key]) -> String {
        typed(&keys.iter().map(|&k| (k, false)).collect::<Vec<_>>())
    }

    #[test]
    fn kana_korean() {
        // こんにちは → 콘니치하
        assert_eq!(plain(&[KeyB, KeyY, KeyI, KeyA, KeyF]), "콘니치하");
        // ありがとう → 아리가토우
        assert_eq!(
            plain(&[Digit3, KeyL, KeyT, BracketLeft, KeyS, Digit4]),
            "아리가토우"
        );
        // きゃ → 캬, しゅ → 슈, うぇ → 웨 (ㅜ + ㅔ joins)
        assert_eq!(typed(&[(KeyG, false), (Digit7, true)]), "캬");
        assert_eq!(typed(&[(KeyD, false), (Digit8, true)]), "슈");
        assert_eq!(typed(&[(Digit4, false), (Digit5, true)]), "웨");
        // ぱ → 파, ば → 바
        assert_eq!(plain(&[KeyF, BracketRight]), "파");
        assert_eq!(plain(&[KeyF, BracketLeft]), "바");
        // っ: 가 + っ + 카 → 가까; がっこう-style ⇧Z then か゛ → 까 stays tense
        assert_eq!(
            typed(&[
                (KeyT, false),
                (BracketLeft, false),
                (KeyZ, true),
                (KeyT, false)
            ]),
            "가까"
        );
        assert_eq!(
            typed(&[(KeyZ, true), (KeyT, false), (BracketLeft, false)]),
            "까"
        );
        // finals: ⇧ん → ㅇ, ⇧む → ㅁ: 카+゛+⇧む → 감; 사 ん → 산
        assert_eq!(
            typed(&[
                (KeyT, false),
                (BracketLeft, false),
                (Backslash, true),
                (KeyX, false)
            ]),
            "감사"
        );
        assert_eq!(typed(&[(KeyX, false), (KeyY, true)]), "상");
        // い joins a lone ㅋ, else stands on ㅇ: 카이
        assert_eq!(plain(&[KeyT, KeyE]), "카이");
        assert_eq!(typed(&[(KeyZ, false), (KeyE, false)]), "키");
        assert_eq!(plain(&[KeyT, KeyZ]), "캌");
        assert_eq!(plain(&[KeyT, IntlYen]), "카ー");
        // ー after ㅔ / ㅖ: the long form. 세ー → 새, 헤ー → 해
        assert_eq!(plain(&[KeyP, IntlYen]), "새");
        assert_eq!(plain(&[Equal, IntlYen]), "해");
        // っ then a small vowel: ㅋ + that vowel
        assert_eq!(typed(&[(KeyZ, true), (Digit3, true)]), "카");
        assert_eq!(typed(&[(KeyZ, true), (Digit7, true)]), "캬");
        // ん then a small vowel starts a syllable: plain ん ㄴ (or ㅇ), ⇧ん ㅇ
        assert_eq!(typed(&[(KeyY, false), (Digit7, true)]), "냐");
        assert_eq!(
            typed(&[(KeyT, false), (KeyY, false), (Digit3, true)]),
            "카나"
        );
        assert_eq!(
            typed(&[(KeyT, false), (KeyY, true), (Digit3, true)]),
            "카아"
        );
        // ⇧の → 의, completed: a following ん doesn't attach. 나라의 = な ら ⇧の
        assert_eq!(
            typed(&[(KeyU, false), (KeyO, false), (KeyK, true)]),
            "나라의"
        );
        assert_eq!(typed(&[(KeyK, true), (KeyY, false)]), "의ㄴ");
        assert_eq!(plain(&[KeyK]), "노");
    }

    fn all(keys: &[(Key, bool)]) -> Vec<String> {
        readings(
            &keys
                .iter()
                .map(|&(k, s)| Press::new(k, s))
                .collect::<Vec<_>>(),
            4096,
        )
    }

    #[test]
    fn readings_keep_every_pronunciation() {
        // く → 쿠 (default) or 크; ぐ → 구 or 그
        assert_eq!(all(&[(KeyH, false)]), ["쿠", "크"]);
        assert_eq!(all(&[(KeyH, false), (BracketLeft, false)]), ["구", "그"]);
        // は ん ぐ く: 한국 among them (ん ㄴ, ぐ 구, く as a final ㄱ)
        let r = all(&[
            (KeyF, false),
            (KeyY, false),
            (KeyH, false),
            (BracketLeft, false),
            (KeyH, false),
        ]);
        assert_eq!(r[0], "한구쿠");
        assert!(r.contains(&"한국".to_string()), "{r:?}");
        assert!(r.contains(&"항국".to_string()));
        // か゛ む さ: 감사 (む as a final ㅁ)
        let r = all(&[
            (KeyT, false),
            (BracketLeft, false),
            (Backslash, false),
            (KeyX, false),
        ]);
        assert_eq!(r[0], "가무사");
        assert!(r.contains(&"감사".to_string()));
        // そ う る: 서울 (そ ㅓ, う 우, る as a final ㄹ)
        let r = all(&[(KeyC, false), (Digit4, false), (Period, false)]);
        assert!(r.contains(&"서울".to_string()), "{r:?}");
        // っ: tense, or a ㅅ final before: か っ か → 카까 or 캇카
        let r = all(&[(KeyT, false), (KeyZ, true), (KeyT, false)]);
        assert_eq!(r, ["카까", "캇카"]);
        assert_eq!(all(&[(KeyY, false), (Digit7, true)]), ["냐", "야"]);
        assert_eq!(all(&[(KeyY, true), (Digit7, true)]), ["야"]);
        // あんにょ⇧んぐはせよ shows 안뇽구하세요; 안녕하세요 is among the readings (ょ ㅕ, ング ㅇ)
        let keys = [
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
        ];
        let r = all(&keys);
        assert_eq!(r[0], "안뇽구하세요");
        assert!(r.contains(&"안녕하세요".to_string()), "{r:?}");
        // the same with plain ん (ㄴ or ㅇ)
        let mut plain_n = keys;
        plain_n[4] = (KeyY, false);
        assert!(all(&plain_n).contains(&"안녕하세요".to_string()));
    }
}
