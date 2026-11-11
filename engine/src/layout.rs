//! Layouts (spec §5): what a key means in each, and how the Korean ones compose.
//!
//! - 0 JIS QWERTY (muhenkan): every key types its JIS character.
//! - 10 JIS KTS (henkan): standard 2-set on the letter keys, JIS symbols elsewhere.
//! - 13 JIS Bada (henkan): `docs/layouts/13-bada.json`, deterministic (docs/layouts/13-bada.md).
//! - 14 JIS 3-set (henkan): `docs/layouts/14-3set-jis.json`, read as it is (docs/layouts/14-3set-jis.md).
//! - 150 Symbols and 151 Unicode (henkan): searches (`search.rs`).
//! - 15 bada roman (henkan): `bada_roman.rs`.
//! - 18 JIS Korean Roman (henkan): `korean_roman.rs`.
//! - 19 JIS Kana Korean (henkan): `kana_korean.rs`.
//! - 20 JIS Kana and 21 JIS Romaji (henkan, with kanji conversion): `japanese.rs`, `kanji.rs`.

use crate::hangul::{cluster, compose, compound};
use crate::key::Key;

pub const QWERTY: u32 = 0;
pub const KTS: u32 = 10;
pub const LEGACY_KOREAN: u32 = 11;
pub const LEGACY_MOONSHOT: u32 = 12;
pub const BADA: u32 = 13;
pub const K3S_JIS: u32 = 14;
pub const BADA_ROMAN: u32 = 15;
pub const SYMBOLS: u32 = 150;
pub const UNICODE: u32 = 151;
pub const KOREAN_ROMAN: u32 = 18;
pub const KANA_KOREAN: u32 = 19;
pub const KANA: u32 = 20;
pub const ROMAJI: u32 = 21;

/// Every layout this build knows, with its display name and whether it composes.
pub const CATALOG: &[(u32, &str, bool)] = &[
    (QWERTY, "QWERTY", false),
    (KTS, "Korean (2-set)", true),
    (LEGACY_KOREAN, "Korean (legacy-korean)", true),
    (LEGACY_MOONSHOT, "Korean (legacy-moonshot)", true),
    (BADA, "Korean (bada)", true),
    (K3S_JIS, "Korean (3-set JIS)", true),
    (BADA_ROMAN, "Korean (bada roman)", true),
    (KOREAN_ROMAN, "Korean (Roman)", true),
    (KANA_KOREAN, "Korean (Kana)", true),
    (KANA, "Japanese (Kana)", true),
    (ROMAJI, "Japanese (Romaji)", true),
    (SYMBOLS, "Symbols", true),
    (UNICODE, "Unicode", true),
];

/// Layouts whose keys spell a search (150, 151): with only these in use, Space is part of it.
pub fn is_search(id: u32) -> bool {
    id == SYMBOLS || id == UNICODE
}

pub fn is_known(id: u32) -> bool {
    CATALOG.iter().any(|c| c.0 == id)
}

pub fn is_henkan(id: u32) -> bool {
    CATALOG.iter().any(|c| c.0 == id && c.2)
}

/// The JIS character of a key on a JIS keyboard (JIS QWERTY), with or without Shift.
pub fn jis_char(key: Key, shift: bool) -> Option<char> {
    use Key::*;
    if key.is_letter() {
        let c = key.code().chars().nth(3)?;
        return Some(if shift { c } else { c.to_ascii_lowercase() });
    }
    let (base, shifted) = match key {
        Digit1 => ('1', '!'),
        Digit2 => ('2', '"'),
        Digit3 => ('3', '#'),
        Digit4 => ('4', '$'),
        Digit5 => ('5', '%'),
        Digit6 => ('6', '&'),
        Digit7 => ('7', '\''),
        Digit8 => ('8', '('),
        Digit9 => ('9', ')'),
        Digit0 => ('0', '0'),
        Minus => ('-', '='),
        Equal => ('^', '~'),
        IntlYen => ('¥', '|'),
        BracketLeft => ('@', '`'),
        BracketRight => ('[', '{'),
        Semicolon => (';', '+'),
        Quote => (':', '*'),
        Backslash => (']', '}'),
        Comma => (',', '<'),
        Period => ('.', '>'),
        Slash => ('/', '?'),
        IntlRo => ('\\', '_'),
        Space => (' ', ' '),
        _ => return None,
    };
    Some(if shift { shifted } else { base })
}

/// What one press means to a Korean layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A consonant: its initial and final forms (either may be missing, e.g. ㄸ has no final).
    Cons {
        ini: Option<char>,
        fin: Option<char>,
    },
    /// A consonant typed with its 중성 ㅏ (JIS Moonshot's ⇧ + consonant).
    ConsA(char),
    Vowel(char),
    /// ⇧ + a vowel (JIS Korean §4.2): starts a syllable on ㅇ, unless an initial waits for it.
    VowelNew(char),
    /// A character taken as it is (a digit or a symbol).
    Lit(char),
}

const fn c(j: char) -> Role {
    Role::Cons {
        ini: Some(j),
        fin: Some(j),
    }
}
const fn v(j: char) -> Role {
    Role::Vowel(j)
}

fn kts_base(key: Key) -> Option<Role> {
    use Key::*;
    Some(match key {
        KeyQ => c('ㅂ'),
        KeyW => c('ㅈ'),
        KeyE => c('ㄷ'),
        KeyR => c('ㄱ'),
        KeyT => c('ㅅ'),
        KeyY => v('ㅛ'),
        KeyU => v('ㅕ'),
        KeyI => v('ㅑ'),
        KeyO => v('ㅐ'),
        KeyP => v('ㅔ'),
        KeyA => c('ㅁ'),
        KeyS => c('ㄴ'),
        KeyD => c('ㅇ'),
        KeyF => c('ㄹ'),
        KeyG => c('ㅎ'),
        KeyH => v('ㅗ'),
        KeyJ => v('ㅓ'),
        KeyK => v('ㅏ'),
        KeyL => v('ㅣ'),
        KeyZ => c('ㅋ'),
        KeyX => c('ㅌ'),
        KeyC => c('ㅊ'),
        KeyV => c('ㅍ'),
        KeyB => v('ㅠ'),
        KeyN => v('ㅜ'),
        KeyM => v('ㅡ'),
        _ => return None,
    })
}

fn kts(key: Key, shift: bool) -> Option<Role> {
    use Key::*;
    if !key.is_letter() {
        return jis_char(key, shift).map(Role::Lit);
    }
    if shift {
        let tense = |j| Role::Cons {
            ini: Some(j),
            fin: None,
        };
        return Some(match key {
            KeyQ => tense('ㅃ'),
            KeyW => tense('ㅉ'),
            KeyE => tense('ㄸ'),
            KeyR => c('ㄲ'),
            KeyT => c('ㅆ'),
            KeyO => v('ㅒ'),
            KeyP => v('ㅖ'),
            _ => kts_base(key)?,
        });
    }
    kts_base(key)
}

/// The number row JIS Korean and JIS Moonshot share: ㅃ ㅉ ㄸ ㄲ ㅆ ㅟ ㅝ ㅚ ㅢ ㅘ, digits on Shift.
fn legacy_number_row(key: Key, shift: bool) -> Option<Role> {
    use Key::*;
    let ini_only = |j| Role::Cons { ini: Some(j), fin: None };
    let base = match key {
        Digit1 => ini_only('ㅃ'), Digit2 => ini_only('ㅉ'), Digit3 => ini_only('ㄸ'), Digit4 => c('ㄲ'), Digit5 => c('ㅆ'),
        Digit6 => v('ㅟ'), Digit7 => v('ㅝ'), Digit8 => v('ㅚ'), Digit9 => v('ㅢ'), Digit0 => v('ㅘ'),
        _ => return None,
    };
    Some(if shift { Role::Lit(key.code().chars().last()?) } else { base })
}

/// Layout 11: JIS Korean Mode A (jis-korean docs/spec.md §3, §4.2).
fn legacy_korean(key: Key, shift: bool) -> Option<Role> {
    use Key::*;
    if let Some(r) = legacy_number_row(key, shift) {
        return Some(r);
    }
    let lit = |a: char, b: char| Some(Role::Lit(if shift { b } else { a }));
    match key {
        Minus => return lit('!', '?'),
        Equal => return lit('(', ')'),
        IntlYen => return lit('—', '^'),
        BracketLeft => return lit('\'', '"'),
        BracketRight => return lit('[', '「'),
        Semicolon => return lit('~', '-'),
        Quote => return lit(':', ';'),
        Backslash => return lit(']', '」'),
        Comma => return lit(',', ','),
        Period => return lit('.', '.'),
        Slash => return lit('·', '·'),
        IntlRo => return lit('\\', '_'),
        _ => {}
    }
    if !shift {
        return kts_base(key);
    }
    // ⇧Q W E R T O P: standard 2-set; ⇧ on the other vowel keys starts a syllable on ㅇ.
    match key {
        KeyQ | KeyW | KeyE | KeyR | KeyT | KeyO | KeyP => kts(key, true),
        _ => match kts_base(key)? {
            Role::Vowel(x) => Some(Role::VowelNew(x)),
            other => Some(other),
        },
    }
}

/// Layout 12: JIS Moonshot Mode A's Korean reading (jis-moonshot docs/moonshot.md §9).
fn legacy_moonshot(key: Key, shift: bool) -> Option<Role> {
    use Key::*;
    if let Some(r) = legacy_number_row(key, shift) {
        return Some(r);
    }
    let lit = |a: char, b: char| Some(Role::Lit(if shift { b } else { a }));
    let slot = |x: char| Some(if shift { Role::VowelNew(x) } else { Role::Vowel(x) });
    match key {
        Minus => return slot('ㅙ'),
        Equal => return lit('^', '~'),
        IntlYen => return lit('¥', '|'),
        BracketLeft => return lit('@', '`'),
        BracketRight => return lit('[', '{'),
        Semicolon => return slot('ㅖ'),
        Quote => return slot('ㅒ'),
        Backslash => return lit(']', '}'),
        Comma => return Some(if shift { Role::Lit(',') } else { Role::Vowel('ㅢ') }),
        Period => return lit('.', '.'),
        Slash => return lit('/', '?'),
        IntlRo => return lit('\\', '_'),
        _ => {}
    }
    let base = kts_base(key)?;
    if !shift {
        return Some(base);
    }
    // ⇧ on a consonant: it and ㅏ (가); ⇧ on a vowel: a syllable on ㅇ (애, 에 on O and P too).
    Some(match base {
        Role::Cons { ini: Some(j), .. } => Role::ConsA(j),
        Role::Vowel(x) => Role::VowelNew(x),
        other => other,
    })
}

/// Layout 13, unshifted (docs/layouts/13-bada.json, docs/layouts/13-bada.md §2).
fn bada_base(key: Key) -> Option<Role> {
    use Key::*;
    let ini = |j| Role::Cons { ini: Some(j), fin: None };
    let fin = |j| Role::Cons { ini: None, fin: Some(j) };
    let ppa = Role::Cons { ini: Some('ㅃ'), fin: Some('ㅄ') };
    Some(match key {
        Digit1 => v('ㅒ'),
        Digit2 => c('ㅆ'),
        Digit3 => ini('ㅉ'),
        Digit4 => c('ㄲ'),
        Digit5 => ini('ㄸ'),
        Digit6 => v('ㅟ'),
        Digit7 => v('ㅝ'),
        Digit8 => v('ㅠ'),
        Digit9 => v('ㅛ'),
        Digit0 => v('ㅘ'),
        Minus => v('ㅙ'),
        Equal => v('ㅞ'),
        IntlYen => fin('ㅂ'),
        KeyQ => ini('ㅃ'),   // initial only: ㅄ as a final is on `]`
        KeyW => c('ㅌ'),
        KeyE => c('ㄴ'),
        KeyR => c('ㅅ'),
        KeyT => c('ㅍ'),
        KeyY => v('ㅚ'),
        KeyU => v('ㅕ'),
        KeyI => v('ㅑ'),
        KeyO => v('ㅔ'),
        KeyP => v('ㅐ'),
        BracketLeft => v('ㅖ'),
        BracketRight => v('ㅒ'),
        KeyA => c('ㄷ'),
        KeyS => c('ㅈ'),
        KeyD => c('ㄱ'),
        KeyF => ini('ㅇ'),
        KeyG => c('ㅁ'),
        KeyH => v('ㅏ'),
        KeyJ => fin('ㅇ'),
        KeyK => fin('ㄴ'),
        KeyL => fin('ㄹ'),
        Semicolon => fin('ㄱ'),
        Quote => c('ㅆ'),
        Backslash => ppa,
        KeyZ => c('ㅊ'),
        KeyX => c('ㅋ'),
        KeyC => c('ㄹ'),
        KeyV => c('ㅎ'),
        KeyB => c('ㅂ'),
        KeyN => v('ㅓ'),
        KeyM => v('ㅣ'),
        Comma => v('ㅡ'),
        Period => v('ㅗ'),
        Slash => v('ㅜ'),
        IntlRo => v('ㅢ'),
        _ => return None,
    })
}

/// Layout 13 with Shift: its shift layer where it has one; else a letter key types as unshifted
/// and a number or symbol key types its JIS character (docs/layouts/13-bada.md §4).
fn bada(key: Key, shift: bool) -> Option<Role> {
    use Key::*;
    if !shift {
        return bada_base(key);
    }
    let fin = |j| Some(Role::Cons { ini: None, fin: Some(j) });
    match key {
        IntlYen => fin('ㄼ'),
        KeyJ => fin('ㅁ'),
        KeyK => fin('ㄶ'),
        KeyL => fin('ㄺ'),
        Semicolon => fin('ㄲ'),
        _ if key.is_letter() => bada_base(key),
        _ => jis_char(key, false).map(Role::Lit),
    }
}

/// Layout 14, from its editor JSON (`docs/layouts/14-3set-jis.json`): 초성 / 중성 / 종성 keys
/// unshifted; ⇧ types what the file says (the key's JIS character).
fn k3s(key: Key, shift: bool) -> Option<Role> {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static TABLE: OnceLock<HashMap<(Key, bool), Role>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let json: serde_json::Value =
            serde_json::from_str(include_str!("../../docs/layouts/14-3set-jis.json")).unwrap_or_default();
        let mut t = HashMap::new();
        for (code, k) in json["keys"].as_object().into_iter().flatten() {
            for (layer, shift) in [("base", false), ("shift", true)] {
                let Some(tok) = k[layer].as_array().and_then(|a| a.first()) else { continue };
                let (Some(text), Some(r)) = (tok["t"].as_str(), tok["r"].as_str()) else { continue };
                let mut chars = text.chars();
                let (Some(c), None) = (chars.next(), chars.next()) else { continue };
                let role = match r {
                    "i" => Role::Cons { ini: Some(c), fin: None },
                    "f" => Role::Cons { ini: None, fin: Some(c) },
                    "c" => Role::Cons { ini: Some(c), fin: Some(c) },
                    "v" => Role::Vowel(c),
                    _ => Role::Lit(c),
                };
                t.insert((Key::from_code(code), shift), role);
            }
        }
        t
    });
    table.get(&(key, shift)).copied()
}

/// A tense initial or final made by doubling (3-set: ㄱ ㄱ → ㄲ).
fn doubled(j: char) -> Option<char> {
    Some(match j {
        'ㄱ' => 'ㄲ',
        'ㄷ' => 'ㄸ',
        'ㅂ' => 'ㅃ',
        'ㅅ' => 'ㅆ',
        'ㅈ' => 'ㅉ',
        _ => return None,
    })
}

/// A Korean layout's rules.
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    /// ㅗ + ㅏ joins into ㅘ (2-set).
    pub join_vowels: bool,
    /// A vowel at the start, after a vowel or after an open syllable starts a syllable on ㅇ.
    pub implied_ieung: bool,
    /// The same vowel key again right after it took a final puts the final back (JIS Korean §4.2).
    pub double_toggle: bool,
    /// A ⇧ + vowel right after ⇧ㄲ / ⇧ㅆ may be Shift held over: a plain vowel (JIS Korean §4.2).
    pub held_shift: bool,
    /// The same initial twice is its tense form, the same final twice too (3-set: ㄱ ㄱ → ㄲ).
    pub double_tense: bool,
}

pub fn rules(id: u32) -> Option<Rules> {
    match id {
        KTS => Some(Rules {
            join_vowels: true,
            implied_ieung: false,
            double_toggle: false,
            held_shift: false,
            double_tense: false,
        }),
        LEGACY_KOREAN => Some(Rules {
            join_vowels: true,
            implied_ieung: true,
            double_toggle: true,
            held_shift: true,
            double_tense: false,
        }),
        LEGACY_MOONSHOT => Some(Rules {
            join_vowels: true,
            implied_ieung: true,
            double_toggle: true,
            held_shift: false,
            double_tense: false,
        }),
        BADA => Some(Rules {
            join_vowels: true,
            implied_ieung: false,
            double_toggle: false,
            held_shift: false,
            double_tense: false,
        }),
        K3S_JIS => Some(Rules {
            join_vowels: true,
            implied_ieung: false,
            double_toggle: false,
            held_shift: false,
            double_tense: true,
        }),
        _ => None,
    }
}

/// What a press means in Korean layout `id`, or None if it means nothing there.
pub fn role(id: u32, key: Key, shift: bool) -> Option<Role> {
    match id {
        KTS => kts(key, shift),
        LEGACY_KOREAN => legacy_korean(key, shift),
        LEGACY_MOONSHOT => legacy_moonshot(key, shift),
        BADA => bada(key, shift),
        K3S_JIS => k3s(key, shift),
        _ => None,
    }
}

/// One staged press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Press {
    pub key: Key,
    pub shift: bool,
    /// Typed with ⌥ (⇧⌥ on a symbol key: its shifted JIS character, §8.1).
    pub alt: bool,
}

impl Press {
    pub fn new(key: Key, shift: bool) -> Press {
        Press {
            key,
            shift,
            alt: false,
        }
    }
}

/// The role of a press in a layout, with ⌥ applied: ⇧⌥ on a non-letter key is its shifted JIS
/// character in every Korean layout.
pub fn press_role(id: u32, p: Press) -> Option<Role> {
    if p.alt {
        return jis_char(p.key, p.shift).map(Role::Lit);
    }
    role(id, p.key, p.shift)
}

#[derive(Clone, Debug, Default)]
struct Syl {
    ini: Option<char>,
    vowel: Option<char>,
    /// Finals with the press index each came from; at most two.
    fins: Vec<(char, usize)>,
    lit: Option<char>,
    /// The ㅇ was implied, not typed or asked for (JIS Korean §4.2): `commit-bare` drops it.
    implied: bool,
}

impl Syl {
    fn text(&self, out: &mut String, drop_implied: bool) {
        if let Some(l) = self.lit {
            out.push(l);
            return;
        }
        // The bare vowel, while the syllable has no final yet (안 stays 안).
        if drop_implied && self.implied && self.fins.is_empty() {
            if let Some(v) = self.vowel {
                out.push(v);
                return;
            }
        }
        let fin = match self.fins.as_slice() {
            [] => None,
            [(a, _)] => Some(*a),
            [(a, _), (b, _)] => cluster(*a, *b),
            _ => None,
        };
        match (self.ini, self.vowel) {
            (Some(l), Some(v)) => match compose(l, v, fin) {
                Some(s) => out.push(s),
                None => {
                    out.push(l);
                    out.push(v);
                    self.fins.iter().for_each(|f| out.push(f.0));
                }
            },
            (Some(l), None) => out.push(l),
            (None, Some(v)) => {
                out.push(v);
                self.fins.iter().for_each(|f| out.push(f.0));
            }
            (None, None) => {}
        }
    }
}

/// The composed text of staged presses in Korean layout `id` (normal composition).
pub fn compose_presses(id: u32, presses: &[Press]) -> String {
    compose_with(id, presses, false)
}

/// As [`compose_presses`], with the implied ㅇ dropped where JIS Korean's F6 dropped it: ㅏ → ㅏ,
/// 가어 → 가ㅓ, 와 → ㅘ; a typed or asked-for ㅇ, and a syllable with a final, stay.
pub fn compose_bare(id: u32, presses: &[Press]) -> String {
    compose_with(id, presses, true)
}

fn compose_with(id: u32, presses: &[Press], drop_implied: bool) -> String {
    if id == KANA_KOREAN {
        return crate::kana_korean::compose_presses(presses);
    }
    if id == BADA_ROMAN {
        return crate::bada_roman::compose_presses(presses);
    }
    if id == K3S_JIS {
        return compose_moa(id, presses);
    }
    let Some(syls) = build(id, presses) else {
        return String::new();
    };
    let mut out = String::new();
    syls.iter().for_each(|s| s.text(&mut out, drop_implied));
    out
}

/// 모아쓰기 (3-set, layout 14): initial, vowel and final have keys of their own, so a syllable is
/// assembled whatever order its jamo come in (ㅏ ㅎ ㄴ, ㅎ ㄴ ㅏ, ㅎ ㅏ ㄴ: 한), keys pressed together
/// included. A jamo whose slot is taken, and that can't join what's there (ㄱ ㄱ → ㄲ, ㅗ ㅏ → ㅘ,
/// ㄹ ㄱ → ㄺ), starts the next syllable.
fn compose_moa(id: u32, presses: &[Press]) -> String {
    #[derive(Default)]
    struct Moa {
        ini: Option<char>,
        vowel: Option<char>,
        fin: Option<char>,
        /// The final was built from two keys (a cluster or a doubling): nothing more joins it.
        fin_full: bool,
    }
    impl Moa {
        fn is_empty(&self) -> bool {
            self.ini.is_none() && self.vowel.is_none() && self.fin.is_none()
        }
        fn text(&self, out: &mut String) {
            match (self.ini, self.vowel) {
                (Some(l), Some(v)) => match compose(l, v, self.fin) {
                    Some(c) => out.push(c),
                    None => out.extend([Some(l), Some(v), self.fin].into_iter().flatten()),
                },
                _ => out.extend([self.ini, self.vowel, self.fin].into_iter().flatten()),
            }
        }
    }
    let r = rules(id).unwrap_or(Rules {
        join_vowels: true, implied_ieung: false, double_toggle: false, held_shift: false, double_tense: true,
    });
    let mut out = String::new();
    let mut cur = Moa::default();
    for &p in presses {
        let Some(role) = press_role(id, p) else { continue };
        match role {
            Role::Cons { ini: Some(l), .. } => {
                match cur.ini {
                    None => cur.ini = Some(l),   // a syllable is never done without its initial
                    Some(have) if have == l && r.double_tense && cur.vowel.is_none() && cur.fin.is_none() && doubled(l).is_some() => {
                        cur.ini = doubled(l)
                    }
                    _ => {
                        cur.text(&mut out);
                        cur = Moa { ini: Some(l), ..Moa::default() };
                    }
                }
            }
            Role::Vowel(v) | Role::VowelNew(v) => match cur.vowel {
                None => cur.vowel = Some(v),
                Some(have) if r.join_vowels && cur.fin.is_none() && compound(have, v).is_some() => cur.vowel = compound(have, v),
                _ => {
                    cur.text(&mut out);
                    cur = Moa { vowel: Some(v), ..Moa::default() };
                }
            },
            Role::Cons { ini: None, fin: Some(f) } => match cur.fin {
                None => cur.fin = Some(f),
                Some(have) if !cur.fin_full => {
                    let joined = cluster(have, f)
                        .or_else(|| if r.double_tense && have == f { doubled(f).filter(|t| matches!(t, 'ㄲ' | 'ㅆ')) } else { None });
                    if let Some(j) = joined {
                        cur.fin = Some(j);
                        cur.fin_full = true;
                    } else {
                        cur.text(&mut out);
                        cur = Moa { fin: Some(f), ..Moa::default() };
                    }
                }
                _ => {
                    cur.text(&mut out);
                    cur = Moa { fin: Some(f), ..Moa::default() };
                }
            },
            Role::Cons { ini: None, fin: None } => {}
            Role::ConsA(l) => {
                cur.text(&mut out);
                cur = Moa { ini: Some(l), vowel: Some('ㅏ'), ..Moa::default() };
            }
            Role::Lit(c) => {
                cur.text(&mut out);
                cur = Moa::default();
                out.push(c);
            }
        }
    }
    if !cur.is_empty() {
        cur.text(&mut out);
    }
    out
}

fn build(id: u32, presses: &[Press]) -> Option<Vec<Syl>> {
    let r = rules(id)?;
    let roles: Vec<Option<Role>> = presses.iter().map(|&p| press_role(id, p)).collect();
    let mut syls: Vec<Syl> = Vec::new();
    // The vowel press that last took a final (its index, and the final), for the double press.
    let mut last_steal: Option<(usize, (char, usize))> = None;
    for (i, role) in roles.iter().enumerate() {
        let Some(mut role) = *role else { continue };
        let prev_steal = last_steal.take();
        // The same vowel key again right after it took a final: the final goes back, and the
        // vowel starts its own syllable on ㅇ (마리 → 말이; JIS Korean §4.2).
        if let (true, Role::Vowel(vw), Some((at, fin))) = (r.double_toggle, role, prev_steal) {
            if at + 1 == i && presses[at].key == presses[i].key && !presses[i].shift {
                syls.pop();
                if let Some(cur) = syls.last_mut() {
                    cur.fins.push(fin);
                }
                syls.push(Syl { ini: Some('ㅇ'), vowel: Some(vw), ..Syl::default() });
                continue;
            }
        }
        if let Role::VowelNew(vw) = role {
            let waiting = syls.last().is_some_and(|c| c.lit.is_none() && c.ini.is_some() && c.vowel.is_none());
            // ⇧ held over from ⇧ㄲ / ⇧ㅆ: a plain vowel, which takes the final as 2-set does.
            let held = r.held_shift
                && i > 0
                && presses[i - 1].shift
                && matches!(roles[i - 1], Some(Role::Cons { ini: Some('ㄲ' | 'ㅆ'), .. }));
            if waiting || held {
                role = Role::Vowel(vw);
            } else {
                syls.push(Syl { ini: Some('ㅇ'), vowel: Some(vw), ..Syl::default() });
                continue;
            }
        }
        match role {
            Role::VowelNew(_) => {}
            Role::Lit(ch) => syls.push(Syl {
                lit: Some(ch),
                ..Syl::default()
            }),
            Role::Cons { ini, fin } => {
                if let (Some(cur), Some(f)) = (syls.last_mut(), fin) {
                    if cur.lit.is_none()
                        && cur.vowel.is_some()
                        && (cur.ini.is_some() || r.implied_ieung)
                    {
                        // 3-set: the same final twice is its tense form (ㄱ ㄱ → ㄲ).
                        if let ([(a, at)], true) = (cur.fins.as_slice(), r.double_tense) {
                            if *a == f && ini.is_none() {
                                if let Some(t) = doubled(f).filter(|t| matches!(t, 'ㄲ' | 'ㅆ')) {
                                    cur.fins = vec![(t, *at)];
                                    continue;
                                }
                            }
                        }
                        let joins = match cur.fins.as_slice() {
                            [] => true,
                            [(a, _)] => cluster(*a, f).is_some(),
                            _ => false,
                        };
                        if joins && cur.ini.is_some() {
                            cur.fins.push((f, i));
                            continue;
                        }
                    }
                }
                // 3-set: the same initial twice, before a vowel, is its tense form (ㄱ ㄱ → ㄲ).
                if let (true, Some(l), Some(cur)) = (r.double_tense, ini, syls.last_mut()) {
                    if cur.lit.is_none() && cur.vowel.is_none() && cur.ini == Some(l) {
                        if let Some(t) = doubled(l) {
                            cur.ini = Some(t);
                            continue;
                        }
                    }
                }
                match ini {
                    Some(l) => syls.push(Syl {
                        ini: Some(l),
                        ..Syl::default()
                    }),
                    None => syls.push(Syl {
                        lit: fin,
                        ..Syl::default()
                    }),
                }
            }
            Role::ConsA(l) => syls.push(Syl {
                ini: Some(l),
                vowel: Some('ㅏ'),
                ..Syl::default()
            }),
            Role::Vowel(vw) => {
                if let Some(cur) = syls.last_mut() {
                    if cur.lit.is_none() {
                        if cur.ini.is_some() && cur.vowel.is_none() {
                            cur.vowel = Some(vw);
                            continue;
                        }
                        if r.join_vowels && cur.fins.is_empty() {
                            if let Some(joined) = cur.vowel.and_then(|a| compound(a, vw)) {
                                cur.vowel = Some(joined);
                                continue;
                            }
                        }
                        // A vowel after a final takes the last final's key as its initial.
                        if cur.vowel.is_some() && !cur.fins.is_empty() {
                            let (_, from) = *cur.fins.last().unwrap();
                            let ini = match roles[from] {
                                Some(Role::Cons { ini, .. }) => ini,
                                _ => None,
                            };
                            if let Some(l) = ini {
                                let taken = cur.fins.pop().unwrap();
                                last_steal = Some((i, taken));
                                syls.push(Syl {
                                    ini: Some(l),
                                    vowel: Some(vw),
                                    ..Syl::default()
                                });
                                continue;
                            }
                        }
                    }
                }
                let ini = if r.implied_ieung { Some('ㅇ') } else { None };
                syls.push(Syl {
                    ini,
                    vowel: Some(vw),
                    implied: r.implied_ieung,
                    ..Syl::default()
                });
            }
        }
    }
    Some(syls)
}

/// Bada's `raw` action (docs/layouts/13-bada.md §6): every key as its jamo, nothing composed.
pub fn raw(id: u32, presses: &[Press]) -> String {
    presses
        .iter()
        .filter_map(|&p| match press_role(id, p)? {
            Role::Cons { ini, fin } => ini.or(fin),
            Role::ConsA(l) => Some(l),
            Role::Vowel(v) | Role::VowelNew(v) => Some(v),
            Role::Lit(ch) => Some(ch),
        })
        .collect()
}

/// A search layout's query (150, 151): letters lowercase (a token is the letter, either case),
/// digits and symbols as their JIS characters, Space a space.
pub fn query(presses: &[Press]) -> String {
    presses
        .iter()
        .filter_map(|p| if p.key == Key::Space { Some(' ') } else { jis_char(p.key, p.shift) })
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The staged presses as JIS QWERTY text.
pub fn qwerty(presses: &[Press]) -> String {
    presses
        .iter()
        .filter_map(|p| jis_char(p.key, p.shift))
        .collect()
}

/// Whether any press means something in layout `id` (it can take the press).
pub fn takes(id: u32, p: Press) -> bool {
    match id {
        QWERTY => jis_char(p.key, p.shift).is_some(),
        KANA_KOREAN => crate::kana_korean::takes(p),
        KOREAN_ROMAN => crate::korean_roman::takes(p),
        BADA_ROMAN => crate::bada_roman::takes(p),
        KANA => crate::japanese::kana_takes(p),
        ROMAJI => crate::japanese::romaji_takes(p),
        SYMBOLS | UNICODE => !p.alt && (p.key == Key::Space || jis_char(p.key, p.shift).is_some()),
        _ => press_role(id, p).is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Key::*;

    fn keys(id: u32, spec: &[(Key, bool)]) -> String {
        let presses: Vec<Press> = spec.iter().map(|&(k, s)| Press::new(k, s)).collect();
        compose_presses(id, &presses)
    }
    fn plain(id: u32, ks: &[Key]) -> String {
        keys(id, &ks.iter().map(|&k| (k, false)).collect::<Vec<_>>())
    }

    #[test]
    fn kts_composes() {
        assert_eq!(plain(KTS, &[KeyG, KeyK, KeyS]), "한");
        assert_eq!(
            plain(KTS, &[KeyG, KeyK, KeyS, KeyR, KeyN, KeyR, KeyD, KeyJ]),
            "한국어"
        );
        assert_eq!(plain(KTS, &[KeyR, KeyH, KeyK]), "과");
        assert_eq!(plain(KTS, &[KeyR, KeyK, KeyQ, KeyT]), "값");
        assert_eq!(plain(KTS, &[KeyR, KeyK, KeyQ, KeyT, KeyK]), "갑사");
        assert_eq!(keys(KTS, &[(KeyR, true), (KeyK, false)]), "까");
    }

    #[test]
    fn legacy_korean_rules() {
        let k = |spec: &[(Key, bool)]| keys(LEGACY_KOREAN, spec);
        let p = |ks: &[Key]| plain(LEGACY_KOREAN, ks);
        // implied ㅇ; vowels still combine; 0 is ㅘ
        assert_eq!(p(&[KeyK]), "아");
        assert_eq!(p(&[KeyH, KeyK]), "와");
        assert_eq!(p(&[KeyR, KeyK, KeyJ]), "가어");
        assert_eq!(p(&[KeyR, KeyH, Digit0]), "고와");
        // the same vowel again keeps the final: 마리 → 말이, 닭아, only the first repeat
        assert_eq!(p(&[KeyA, KeyK, KeyF, KeyL]), "마리");
        assert_eq!(p(&[KeyA, KeyK, KeyF, KeyL, KeyL]), "말이");
        assert_eq!(p(&[KeyE, KeyK, KeyF, KeyR, KeyK, KeyK]), "닭아");
        assert_eq!(p(&[KeyA, KeyK, KeyF, KeyL, KeyL, KeyL]), "말이이");
        assert_eq!(p(&[KeyA, KeyK, KeyF, KeyL, KeyK]), "마리아");
        // ⇧ + vowel starts a syllable on ㅇ
        assert_eq!(k(&[(KeyA, false), (KeyK, false), (KeyF, false), (KeyL, true)]), "말이");
        assert_eq!(k(&[(KeyH, false), (KeyL, true)]), "오이");
        assert_eq!(k(&[(KeyR, false), (KeyK, false), (KeyS, false), (KeyH, true), (KeyK, false)]), "간와");
        // held Shift after ⇧ㄲ / ⇧ㅆ: 가까이
        assert_eq!(k(&[(KeyR, false), (KeyK, false), (KeyR, true), (KeyK, true), (KeyD, false), (KeyL, false)]), "가까이");
        // O P keep ㅒ ㅖ; number row and symbols
        assert_eq!(k(&[(KeyR, false), (KeyO, true)]), "걔");
        assert_eq!(p(&[Digit5, KeyK]), "싸");
        assert_eq!(k(&[(KeyG, false), (KeyK, false), (Minus, false), (Minus, true)]), "하!?");
    }

    #[test]
    fn bare_drops_only_the_implied_ieung() {
        let bare = |spec: &[(Key, bool)]| compose_bare(LEGACY_KOREAN, &spec.iter().map(|&(k, s)| Press::new(k, s)).collect::<Vec<_>>());
        assert_eq!(bare(&[(KeyK, false)]), "ㅏ");
        assert_eq!(bare(&[(KeyR, false), (KeyK, false), (KeyJ, false)]), "가ㅓ");
        assert_eq!(bare(&[(KeyH, false), (KeyK, false)]), "ㅘ");
        assert_eq!(bare(&[(KeyK, false), (KeyS, false)]), "안");
        // typed ㅇ, the double press and ⇧ + vowel stay
        assert_eq!(bare(&[(KeyD, false), (KeyK, false)]), "아");
        assert_eq!(bare(&[(KeyA, false), (KeyK, false), (KeyF, false), (KeyL, false), (KeyL, false)]), "말이");
        assert_eq!(bare(&[(KeyH, false), (KeyL, true)]), "ㅗ이");
    }

    #[test]
    fn legacy_moonshot_rules() {
        let k = |spec: &[(Key, bool)]| keys(LEGACY_MOONSHOT, spec);
        // ⇧ consonant = it + ㅏ; ⇧ vowel (O P too) = a syllable on ㅇ; no Shift held over
        assert_eq!(k(&[(KeyR, true)]), "가");
        assert_eq!(k(&[(KeyG, false), (KeyK, false), (KeyS, true)]), "하나");
        assert_eq!(k(&[(KeyR, false), (KeyK, false), (KeyS, false), (KeyP, true)]), "간에");
        assert_eq!(k(&[(KeyR, true), (KeyK, true)]), "가아");
        // slot keys: ㄷ − → 돼, 말 ⇧− → 말왜, ㄱ ; → 계, ㅎ , → 희
        assert_eq!(k(&[(KeyE, false), (Minus, false)]), "돼");
        assert_eq!(k(&[(KeyA, false), (KeyK, false), (KeyF, false), (Minus, true)]), "말왜");
        assert_eq!(k(&[(KeyR, false), (Semicolon, false)]), "계");
        assert_eq!(k(&[(KeyG, false), (Comma, false)]), "희");
        assert_eq!(k(&[(KeyG, false), (KeyK, false), (Slash, true)]), "하?");
    }

    #[test]
    fn bada_composes() {
        // 한: ㅎ(V) ㅏ(H) ㄴ(K, final only)
        assert_eq!(plain(BADA, &[KeyV, KeyH, KeyK]), "한");
        // 안녕: ㅇ is typed (no implied ㅇ)
        assert_eq!(plain(BADA, &[KeyF, KeyH, KeyK, KeyE, KeyU, KeyJ]), "안녕");
        assert_eq!(plain(BADA, &[KeyH, KeyK]), "ㅏㄴ");
        // an initial-or-final key: the final, until a vowel takes it (2-set): 간, 가나
        assert_eq!(plain(BADA, &[KeyD, KeyH, KeyE]), "간");
        assert_eq!(plain(BADA, &[KeyD, KeyH, KeyE, KeyH]), "가나");
        // a final-only key never moves: 간 + ㅏ stays 간ㅏ
        assert_eq!(plain(BADA, &[KeyD, KeyH, KeyK, KeyH]), "간ㅏ");
        // 것, 것이, 거시
        assert_eq!(plain(BADA, &[KeyD, KeyN, KeyR]), "것");
        assert_eq!(plain(BADA, &[KeyD, KeyN, KeyR, KeyF, KeyM]), "것이");
        assert_eq!(plain(BADA, &[KeyD, KeyN, KeyR, KeyM]), "거시");
        // vowels join as in 2-set; compound vowels have keys too
        assert_eq!(plain(BADA, &[KeyB, Period, KeyH]), "봐");
        assert_eq!(plain(BADA, &[KeyB, Digit0]), "봐");
        // double finals: two keys, or one ⇧ key
        assert_eq!(plain(BADA, &[KeyR, KeyH, KeyL, KeyG]), "삶");
        assert_eq!(keys(BADA, &[(KeyG, false), (KeyH, false), (KeyK, true)]), "많");
        assert_eq!(keys(BADA, &[(KeyA, false), (KeyH, false), (KeyL, true)]), "닭");
        assert_eq!(keys(BADA, &[(KeyB, false), (KeyH, false), (IntlYen, true)]), "밟");
        assert_eq!(keys(BADA, &[(KeyR, false), (KeyH, false), (KeyJ, true)]), "삼");
        assert_eq!(keys(BADA, &[(KeyD, false), (KeyH, false), (Semicolon, true)]), "갂");
        // Q is ㅃ only, even after a vowel (아빠, 오빠); `]` is interlaced: ㅄ after a vowel (값)
        assert_eq!(plain(BADA, &[KeyF, KeyH, KeyQ, KeyH]), "아빠");
        assert_eq!(plain(BADA, &[KeyF, KeyH, KeyQ]), "아ㅃ");
        assert_eq!(plain(BADA, &[KeyD, KeyH, Backslash]), "값");
        assert_eq!(plain(BADA, &[KeyD, KeyH, Backslash, KeyH]), "가빠");
        // ⇧. and ⇧/ are punctuation (ㅛ ㅠ are on 9 and 8)
        assert_eq!(keys(BADA, &[(KeyD, false), (KeyH, false), (Period, true)]), "가.");
        assert_eq!(keys(BADA, &[(KeyD, false), (KeyH, false), (Slash, true)]), "가/");
        // ⇧ elsewhere: a letter types as unshifted, a number or symbol key its JIS character
        assert_eq!(keys(BADA, &[(KeyV, true), (KeyH, false)]), "하");
        assert_eq!(keys(BADA, &[(KeyD, false), (KeyH, false), (Digit1, true)]), "가1");
        assert_eq!(keys(BADA, &[(KeyD, false), (KeyH, false), (Quote, true)]), "가:");
    }

    #[test]
    fn three_set_jis_composes() {
        let p = |k| Press::new(k, false);
        let t = |ks: &[Key]| compose_presses(K3S_JIS, &ks.iter().map(|&k| p(k)).collect::<Vec<_>>());
        // 한: ㅎ(M) ㅏ(F) ㄴ(S, final)
        assert_eq!(t(&[KeyM, KeyF, KeyS]), "한");
        // a final never moves: 한 + ㅏ → 한ㅏ; 하나 needs ㄴ's initial key (H)
        assert_eq!(t(&[KeyM, KeyF, KeyS, KeyF]), "한ㅏ");
        assert_eq!(t(&[KeyM, KeyF, KeyH, KeyF]), "하나");
        // doubling: ㄱ ㄱ ㅏ → 까; final ㄱ ㄱ → ㄲ (밖)
        assert_eq!(t(&[KeyK, KeyK, KeyF]), "까");
        assert_eq!(t(&[Semicolon, KeyF, KeyX, KeyX]), "밖");
        // compound vowels join: ㄱ ㅗ ㅏ → 과; double finals cluster: ㄷ ㅏ ㄹ ㄱ → 닭
        assert_eq!(t(&[KeyK, KeyV, KeyF]), "과");
        assert_eq!(t(&[KeyU, KeyF, KeyW, KeyX]), "닭");
        // 모아쓰기: any order within a syllable
        assert_eq!(t(&[KeyF, KeyM, KeyS]), "한");   // ㅏ ㅎ ㄴ
        assert_eq!(t(&[KeyM, KeyS, KeyF]), "한");   // ㅎ ㄴ ㅏ
        assert_eq!(t(&[KeyS, KeyF, KeyM]), "한");   // ㄴ ㅏ ㅎ
        assert_eq!(t(&[KeyF, KeyM, KeyS, KeyK, KeyB, KeyX]), "한국");
        assert_eq!(t(&[KeyM, KeyF, KeyH, KeyF, KeyM]), "하나ㅎ");   // a filled slot starts the next syllable
        // ⇧: the JIS character
        assert_eq!(keys(K3S_JIS, &[(KeyM, false), (KeyF, false), (Digit1, true)]), "하!");
        assert_eq!(keys(K3S_JIS, &[(KeyQ, true)]), "Q");
    }

    #[test]
    fn bada_is_deterministic() {
        let p = |k| Press::new(k, false);
        // consonants alone are jamo
        assert_eq!(compose_presses(BADA, &[p(KeyD), p(KeyR)]), "ㄱㅅ");
        assert_eq!(raw(BADA, &[p(KeyV), p(KeyH), p(KeyK)]), "ㅎㅏㄴ");
    }
}
