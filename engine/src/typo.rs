//! Typo suggestions for layout 13 (docs/layouts/13-bada.md §5): a noisy channel.
//!
//! - **Language model:** a jamo 4-gram model over Korean words (`ko-jamo-4gram.bin`, built by
//!   `tools/build_jamo_lm.py`), Witten–Bell interpolated. It scores how Korean a reading looks.
//! - **Channel:** the slips layout 13 invites, each with a cost: the same consonant on its other key
//!   (E / K for ㄴ: initial-or-final versus final only), D for the final ㅇ (J), Shift missed or
//!   extra on a key with a ⇧ layer, the key next to it on the same row, and **NKRO typing**: keys
//!   of a fast roll registered out of order (two swapped, or three reordered).
//!
//! A reading one slip away from what was typed is suggested when it's likely enough next to the
//! typed one. Suggestions never replace what was typed: the engine lists them under it.

use crate::key::Key;
use crate::layout::{self, Press, Role, BADA};

/// log10 costs of each slip.
const COST_SAME_JAMO: f64 = -1.3;
const COST_NAMED: f64 = -0.7;
const COST_SHIFT: f64 = -2.0;
const COST_NEAR: f64 = -2.5;
/// NKRO typing: keys pressed in a fast roll registered out of order. Two keys on different hands
/// swap most easily, two on one hand less, three in a roll less again; one finger can't overlap.
const COST_NKRO_HANDS: f64 = -0.8;
const COST_NKRO_HAND: f64 = -1.3;
const COST_NKRO_THREE: f64 = -1.8;
/// log10 P(no slip) for the reading as typed.
const COST_NONE: f64 = -0.05;
/// A suggestion must be at least this likely next to the typed reading (log10 ratio).
pub const THRESHOLD: f64 = -1.0;

/// Slips the owner named: typing the first key when the second was meant (D for the final ㅇ, J).
const NAMED: &[(Key, Key)] = &[(Key::KeyD, Key::KeyJ)];

/// Rows of the 48 kana-space keys, for next-key slips.
const ROWS: [&[Key]; 4] = {
    use Key::*;
    [
        &[Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Digit0, Minus, Equal, IntlYen],
        &[KeyQ, KeyW, KeyE, KeyR, KeyT, KeyY, KeyU, KeyI, KeyO, KeyP, BracketLeft, BracketRight],
        &[KeyA, KeyS, KeyD, KeyF, KeyG, KeyH, KeyJ, KeyK, KeyL, Semicolon, Quote, Backslash],
        &[KeyZ, KeyX, KeyC, KeyV, KeyB, KeyN, KeyM, Comma, Period, Slash, IntlRo],
    ]
};

const START: u8 = 1;
const END: u8 = 2;
const OTHER: u8 = 3;
const VOCAB: f64 = 121.0;

/// The jamo 4-gram model.
pub struct JamoLm {
    /// (key, count), sorted by key.
    grams: Vec<(u32, u32)>,
    /// (history key, total count, distinct next symbols), sorted by key.
    hists: Vec<(u32, u32, u32)>,
}

fn fold(ids: &[u8]) -> u32 {
    ids.iter().fold(0, |k, &t| (k << 7) | t as u32)
}

/// Token ids of a text (tools/build_jamo_lm.py).
fn ids(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for ch in text.chars() {
        let o = ch as u32;
        match o {
            0xAC00..=0xD7A3 => {
                let n = o - 0xAC00;
                out.push(4 + (n / 588) as u8);
                out.push(23 + ((n % 588) / 28) as u8);
                if n % 28 > 0 {
                    out.push(43 + (n % 28) as u8);
                }
            }
            0x3131..=0x3163 => out.push(71 + (o - 0x3131) as u8),
            _ => out.push(OTHER),
        }
    }
    out
}

impl JamoLm {
    pub fn parse(bytes: &[u8]) -> Option<JamoLm> {
        let mut at = 0;
        let mut u32_ = || {
            let v = u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?);
            at += 4;
            Some(v)
        };
        if u32_()? != u32::from_le_bytes(*b"BJLM") || u32_()? != 1 {
            return None;
        }
        let n = u32_()? as usize;
        let mut grams = Vec::with_capacity(n);
        for _ in 0..n {
            grams.push((u32_()?, u32_()?));
        }
        let m = u32_()? as usize;
        let mut hists = Vec::with_capacity(m);
        for _ in 0..m {
            hists.push((u32_()?, u32_()?, u32_()?));
        }
        Some(JamoLm { grams, hists })
    }

    fn count(&self, key: u32) -> f64 {
        self.grams.binary_search_by_key(&key, |g| g.0).map_or(0.0, |i| self.grams[i].1 as f64)
    }

    /// P(next | up to three symbols before it), Witten–Bell interpolated.
    fn prob(&self, before: &[u8], next: u8) -> f64 {
        let mut p = 1.0 / VOCAB;
        for n in 0..=before.len() {
            let h = &before[before.len() - n..];
            let Ok(i) = self.hists.binary_search_by_key(&fold(h), |x| x.0) else { break };
            let (_, total, distinct) = self.hists[i];
            let mut g = h.to_vec();
            g.push(next);
            let c = self.count(fold(&g));
            p = (c + distinct as f64 * p) / (total as f64 + distinct as f64);
        }
        p
    }

    /// log10 P(text as one word).
    pub fn logprob(&self, text: &str) -> f64 {
        let mut toks = vec![START; 3];
        toks.extend(ids(text));
        toks.push(END);
        (3..toks.len()).map(|i| self.prob(&toks[i - 3..i], toks[i]).log10()).sum()
    }
}

/// What the presses may have been meant as, one slip away, with the slip's cost. `typed_from`
/// turns the one-way slips around: what the presses may come out as when typed with a slip.
fn variants(presses: &[Press], typed_from: bool) -> Vec<(Vec<Press>, f64)> {
    let role = |p: Press| layout::press_role(BADA, p);
    let jamo = |r: Option<Role>| match r {
        Some(Role::Cons { ini, fin }) => ini.or(fin),
        _ => None,
    };
    // Every press layout 13 has, for "the same consonant on another key".
    let all: Vec<Press> = Key::ALL
        .iter()
        .filter(|k| k.is_kana_space())
        .flat_map(|&k| [Press::new(k, false), Press::new(k, true)])
        .filter(|&p| matches!(role(p), Some(Role::Cons { .. })))
        .collect();
    let mut out = Vec::new();
    let mut with = |i: usize, q: Press, cost: f64| {
        let mut v = presses.to_vec();
        v[i] = q;
        out.push((v, cost));
    };
    for (i, &p) in presses.iter().enumerate() {
        if p.alt {
            continue;
        }
        let r = role(p);
        if let Some(j) = jamo(r) {
            for &q in &all {
                if q != p && jamo(role(q)) == Some(j) && role(q) != r {
                    with(i, q, COST_SAME_JAMO);
                }
            }
        }
        for &(typed, meant) in NAMED {
            let (from, to) = if typed_from { (meant, typed) } else { (typed, meant) };
            if p.key == from {
                with(i, Press { key: to, ..p }, COST_NAMED);
            }
        }
        // Shift missed or extra, where ⇧ types another jamo (not a symbol).
        let other = Press { shift: !p.shift, ..p };
        let jamo_role = |r: Option<Role>| matches!(r, Some(Role::Cons { .. } | Role::Vowel(_)));
        if jamo_role(r) && jamo_role(role(other)) && role(other) != r {
            with(i, other, COST_SHIFT);
        }
        for row in ROWS {
            if let Some(at) = row.iter().position(|&k| k == p.key) {
                for n in [at.wrapping_sub(1), at + 1] {
                    if let Some(&k) = row.get(n) {
                        with(i, Press { key: k, ..p }, COST_NEAR);
                    }
                }
            }
        }
    }
    // NKRO typing: two neighbouring presses swapped, or three reordered.
    for i in 1..presses.len() {
        let (a, b) = (presses[i - 1], presses[i]);
        let (Some(fa), Some(fb)) = (finger(a.key), finger(b.key)) else { continue };
        if a == b || fa == fb {
            continue;
        }
        let mut v = presses.to_vec();
        v.swap(i - 1, i);
        out.push((v, if fa.0 != fb.0 { COST_NKRO_HANDS } else { COST_NKRO_HAND }));
    }
    for i in 2..presses.len() {
        let w = [presses[i - 2], presses[i - 1], presses[i]];
        let f: Vec<_> = w.iter().filter_map(|p| finger(p.key)).collect();
        if f.len() < 3 || f[0] == f[1] || f[1] == f[2] || f[0] == f[2] {
            continue;
        }
        // The two rotations (abc → bca, cab) and the reversal (cba); single swaps are above.
        for order in [[1, 2, 0], [2, 0, 1], [2, 1, 0]] {
            let mut v = presses.to_vec();
            for (j, &o) in order.iter().enumerate() {
                v[i - 2 + j] = w[o];
            }
            if v != presses {
                out.push((v, COST_NKRO_THREE));
            }
        }
    }
    out
}

/// (hand, finger) of a key in touch typing on a JIS board: hand 0 left, 1 right; fingers 0 pinky to
/// 3 index (the index fingers take two columns each).
fn finger(key: Key) -> Option<(u8, u8)> {
    use Key::*;
    Some(match key {
        Digit1 | KeyQ | KeyA | KeyZ => (0, 0),
        Digit2 | KeyW | KeyS | KeyX => (0, 1),
        Digit3 | KeyE | KeyD | KeyC => (0, 2),
        Digit4 | Digit5 | KeyR | KeyT | KeyF | KeyG | KeyV | KeyB => (0, 3),
        Digit6 | Digit7 | KeyY | KeyU | KeyH | KeyJ | KeyN | KeyM => (1, 3),
        Digit8 | KeyI | KeyK | Comma => (1, 2),
        Digit9 | KeyO | KeyL | Period => (1, 1),
        Digit0 | Minus | Equal | IntlYen | KeyP | BracketLeft | BracketRight | Semicolon | Quote | Backslash
        | Slash | IntlRo => (1, 0),
        _ => return None,
    })
}

/// Readings one slip away from the presses worth suggesting: likelihood next to the typed reading at
/// least [`THRESHOLD`], best first, at most `limit`.
pub fn suggest(lm: &JamoLm, presses: &[Press], limit: usize) -> Vec<(String, f64)> {
    let mut out: Vec<(String, f64)> = scored(lm, presses).into_iter().filter(|(_, s)| *s >= THRESHOLD).collect();
    out.truncate(limit);
    out
}

fn loose(text: &str) -> usize {
    text.chars().filter(|c| ('\u{3131}'..='\u{3163}').contains(c)).count()
}

/// Every reading one slip away, best first, with its log10 likelihood next to the typed reading.
pub fn scored(lm: &JamoLm, presses: &[Press]) -> Vec<(String, f64)> {
    if presses.is_empty() || !presses.iter().all(|&p| layout::takes(BADA, p)) {
        return Vec::new();
    }
    let typed = layout::compose_presses(BADA, presses);
    let base = lm.logprob(&typed) + COST_NONE;
    let mut best: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for (v, cost) in variants(presses, false) {
        let text = layout::compose_presses(BADA, &v);
        // A suggestion is whole syllables, or at least fewer loose jamo than what was typed.
        if text == typed || text.is_empty() || (loose(&text) > 0 && loose(&text) >= loose(&typed)) {
            continue;
        }
        let score = lm.logprob(&text) + cost - base;
        let e = best.entry(text).or_insert(f64::NEG_INFINITY);
        *e = e.max(score);
    }
    let mut out: Vec<(String, f64)> = best.into_iter().collect();
    out.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

/// The keys that type a word in layout 13 (initial, vowel and final keys; ㅇ typed), or None if a
/// character has none. For tests and the evaluation (examples/typo_eval.rs).
pub fn bada_keys(word: &str) -> Option<Vec<Press>> {
    use Key::*;
    let p = |k| Press::new(k, false);
    let s = |k| Press::new(k, true);
    let mut out = Vec::new();
    for ch in word.chars() {
        let (ini, vowel, fin) = crate::hangul::decompose(ch)?;
        out.push(p(match ini {
            'ㄱ' => KeyD, 'ㄲ' => Digit4, 'ㄴ' => KeyE, 'ㄷ' => KeyA, 'ㄸ' => Digit5, 'ㄹ' => KeyC,
            'ㅁ' => KeyG, 'ㅂ' => KeyB, 'ㅃ' => KeyQ, 'ㅅ' => KeyR, 'ㅆ' => Quote, 'ㅇ' => KeyF,
            'ㅈ' => KeyS, 'ㅉ' => Digit3, 'ㅊ' => KeyZ, 'ㅋ' => KeyX, 'ㅌ' => KeyW, 'ㅍ' => KeyT,
            'ㅎ' => KeyV, _ => return None,
        }));
        out.push(p(match vowel {
            'ㅏ' => KeyH, 'ㅐ' => KeyP, 'ㅑ' => KeyI, 'ㅒ' => Digit1, 'ㅓ' => KeyN, 'ㅔ' => KeyO,
            'ㅕ' => KeyU, 'ㅖ' => BracketLeft, 'ㅗ' => Period, 'ㅘ' => Digit0, 'ㅙ' => Minus,
            'ㅚ' => KeyY, 'ㅛ' => Digit9, 'ㅜ' => Slash, 'ㅝ' => Digit7, 'ㅞ' => Equal, 'ㅟ' => Digit6,
            'ㅠ' => Digit8, 'ㅡ' => Comma, 'ㅢ' => IntlRo, 'ㅣ' => KeyM, _ => return None,
        }));
        let Some(f) = fin else { continue };
        out.extend(match f {
            'ㅇ' => vec![p(KeyJ)], 'ㄴ' => vec![p(KeyK)], 'ㄹ' => vec![p(KeyL)], 'ㄱ' => vec![p(Semicolon)],
            'ㅂ' => vec![p(IntlYen)], 'ㅁ' => vec![s(KeyJ)], 'ㄶ' => vec![s(KeyK)], 'ㄺ' => vec![s(KeyL)],
            'ㄲ' => vec![s(Semicolon)], 'ㄼ' => vec![s(IntlYen)], 'ㅅ' => vec![p(KeyR)], 'ㅆ' => vec![p(Quote)],
            'ㄷ' => vec![p(KeyA)], 'ㅈ' => vec![p(KeyS)], 'ㅊ' => vec![p(KeyZ)], 'ㅋ' => vec![p(KeyX)],
            'ㅌ' => vec![p(KeyW)], 'ㅍ' => vec![p(KeyT)], 'ㅎ' => vec![p(KeyV)], 'ㅄ' => vec![p(Backslash)],
            'ㄳ' => vec![p(Semicolon), p(KeyR)], 'ㄵ' => vec![p(KeyK), p(KeyS)], 'ㄻ' => vec![p(KeyL), p(KeyG)],
            'ㄽ' => vec![p(KeyL), p(KeyR)], 'ㄾ' => vec![p(KeyL), p(KeyW)], 'ㄿ' => vec![p(KeyL), p(KeyT)],
            'ㅀ' => vec![p(KeyL), p(KeyV)], _ => return None,
        });
    }
    Some(out)
}

/// The presses as they may come out with one slip, with its cost (for the evaluation).
pub fn slips(presses: &[Press]) -> Vec<(Vec<Press>, f64)> {
    variants(presses, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        for w in ["감사합니다", "안녕하세요", "것이", "많아요", "닭", "밟아", "앉아", "삶", "의사", "괜찮아"] {
            let k = bada_keys(w).unwrap();
            assert_eq!(layout::compose_presses(BADA, &k), w, "{w}");
        }
    }

    #[test]
    fn slips_include_the_named_and_role_swaps() {
        // 가 + D (ㄱ) when the final ㅇ (J) was meant
        let typed = [Press::new(Key::KeyD, false), Press::new(Key::KeyH, false), Press::new(Key::KeyD, false)];
        let v = variants(&typed, false);
        let texts: Vec<String> = v.iter().map(|(p, _)| layout::compose_presses(BADA, p)).collect();
        assert!(texts.contains(&"강".to_string()), "{texts:?}");
        // K (final ㄴ) where E (initial ㄴ) was meant: 간ㅏ → 가나
        let typed: Vec<Press> = [Key::KeyD, Key::KeyH, Key::KeyK, Key::KeyH].iter().map(|&k| Press::new(k, false)).collect();
        let texts: Vec<String> = variants(&typed, false).iter().map(|(p, _)| layout::compose_presses(BADA, p)).collect();
        assert!(texts.contains(&"가나".to_string()), "{texts:?}");
    }
}
