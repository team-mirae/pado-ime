//! Builds engine/data/key-models.bin, the key models of the layout-set ranker (src/keymodel.rs):
//! for each layout that can share a set (0, 10–16, 18, 20, 21), an n-gram model over the keys
//! pressed when its language is typed on it.
//!
//! Reads:
//! - engine/tools/cache/keymodel/lists.tsv (engine/tools/build_keymodel.py, which runs this):
//!   English words and tokens, Japanese phrases and words with their readings, Korean law words,
//!   each with its count; SHARES says how much of a model each source is;
//! - engine/data/ko-words.tsv: Korean words with their counts;
//! - docs/layouts/NN-name.json: each layout's keys, rules and spelling tables;
//! - engine/data/*: the engine's data, to check the keys against the engine.
//!
//! Every item is typed from the layout's JSON, the way the typing robot types it (tests/robot.rs,
//! docs/robot.md), and other ways a person may:
//! - English on 0: key by key.
//! - Korean on 10–14 and 16: syllable by syllable. The robot's plan (the first way that composes the word
//!   so far, fewest presses first) has ROBOT of a word's weight; the rest goes to ways drawn for
//!   each syllable from all its ways, a way less likely the more presses and Shift it takes (the
//!   final-only keys of 13 where the robot takes the first key that works, ㅗ ㅏ for ㅘ, a typed ㅇ
//!   where 11 and 12 imply it). Every way is checked to compose the word.
//! - Korean on 15 and 18: romanized from the spelling table, run together or with a break
//!   between syllables: the one the engine shows the word for (asked of the engine for the
//!   commonest words, of the letters' own reading for the rest) has most of the weight.
//! - Korean on all of them: also with a full stop, comma, question or exclamation mark after
//!   the word (MARKS), on the layout's key for it.
//! - Japanese on 20: kana by kana. On 21: in romaji, the table's spelling (the robot's) with most
//!   of the weight, the rest to ん typed as one n before a consonant and to kunrei spellings
//!   (si, ti, tu), each checked to read as the kana.
//!
//! Each run of keys is counted between two boundaries, orders 1 to ORDER, smoothed Witten–Bell
//! (interpolated); n-grams the backoff already gives nearly as well are left out, by a threshold
//! on what listing one adds to the likelihood of the counts, set so that the file is SIZE bytes,
//! and the backoff weights are set again so every history's probabilities sum to one.
//!
//! Prints how often each layout's held-out items are read as that layout against every layout of
//! another language (the items of split `test`, and one Korean word in HELD, left out of a first
//! build made for this), then writes the models from everything. Deterministic: the ways are
//! drawn from a hash of the word, the tables written in order.
//!
//!     cargo run --release --example keymodel [-- --out FILE --size BYTES]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use bada::engine::Mode;
use bada::keymodel::{history_key, symbol, KeyModels, ALPHABET, BOUNDARY};
use bada::layout::{compose_presses, Press};
use bada::{Data, Engine, Key, Mods, Settings};
use serde_json::Value;

const A: usize = ALPHABET;

const ORDER: usize = 4;
/// The file's size, all models together.
const SIZE: usize = 3_600_000;
const LAYOUTS: [u32; 11] = [0, 10, 11, 12, 13, 14, 15, 16, 18, 20, 21];
/// Each model is trained on this many items' worth of counts, whatever its corpora's sizes: the
/// smoothing gives unseen keys more when a model has seen less.
const MASS: f64 = 2.0e7;
/// One Korean word in HELD is held out of the first build (by a hash of the word).
const HELD: u64 = 20;
/// Korean words checked on the engine for layouts 15 and 18 (the commonest).
const CHECKED: usize = 60_000;
/// Rows of a converting layout's menu the robot accepts a word in.
const TOP: usize = 5;
/// Plans kept per syllable, as the robot.
const BEAM: usize = 4;

/// The share of a model's counts each source gets: (language, source, kind, share).
const SHARES: &[(&str, &str, &str, f64)] = &[
    ("en", "tatoeba", "word", 0.34),
    ("en", "tatoeba", "token", 0.06),
    ("en", "leipzig", "word", 0.29),
    ("en", "leipzig", "token", 0.05),
    ("en", "us-law", "word", 0.18),
    ("en", "us-law", "token", 0.03),
    ("en", "list", "word", 0.05),
    ("ja", "tatoeba", "phrase", 0.40),
    ("ja", "tatoeba", "punct", 0.06),
    ("ja", "tatoeba", "word", 0.16),
    ("ja", "law", "phrase", 0.26),
    ("ja", "law", "punct", 0.04),
    ("ja", "law", "word", 0.08),
    ("ko", "words", "word", 0.88),
    ("ko", "law", "word", 0.12),
];
/// Korean on 10–14 and 16: the robot's plan has this much of a word's weight; the rest is shared by
/// the ways drawn for it.
const ROBOT: f64 = 0.6;
/// Ways drawn for a word: more for the commonest words (by rank), whose counts weigh most.
const DRAWN: [(usize, usize); 3] = [(30_000, 6), (300_000, 2), (usize::MAX, 1)];
/// A way to type a syllable is this many times as likely for each press more than the robot's
/// way takes, and for each press with Shift more.
const LONGER: f64 = 0.4;
const SHIFTED: f64 = 0.6;
/// Korean on 15 and 18: the spelling the robot picks, and the other one (run together / broken).
const SPELLINGS: [f64; 2] = [0.92, 0.08];
/// Korean on every layout: how often a word is typed with one of these after it, on the layout's
/// key for it. The word lists have no punctuation; without it a full stop after a Korean word
/// would look like another language's. For the MARKED commonest words (enough to learn how
/// words end).
const MARKS: [(char, f64); 4] = [('.', 0.06), (',', 0.04), ('?', 0.01), ('!', 0.005)];
const MARKED: usize = 200_000;
/// Japanese on 21: the table's spelling (the robot's), then (kunrei, one n for ん): the shares.
const ROMAJI: [((bool, bool), f64); 4] = [((false, false), 0.6), ((false, true), 0.25), ((true, false), 0.05), ((true, true), 0.1)];
/// Spellings of kunrei-shiki that the layout's table (Hepburn) spells otherwise.
const KUNREI: &[(&str, &str)] = &[
    ("し", "si"), ("ち", "ti"), ("つ", "tu"), ("ふ", "hu"), ("じ", "zi"), ("しゃ", "sya"), ("しゅ", "syu"), ("しょ", "syo"),
    ("ちゃ", "tya"), ("ちゅ", "tyu"), ("ちょ", "tyo"), ("じゃ", "zya"), ("じゅ", "zyu"), ("じょ", "zyo"),
];

type P = (Key, bool);

// ---------- layouts (as tests/robot.rs reads them) ----------

struct Tok {
    t: String,
    r: String,
}

struct Layout {
    id: u32,
    json: Value,
    /// Every token with the press that types it, unshifted first.
    toks: Vec<(P, Tok)>,
}

impl Layout {
    fn rule(&self, name: &str) -> bool {
        self.json["rules"][name].as_bool().unwrap_or(false)
    }
    fn presses(&self, t: &str, roles: &[&str]) -> Vec<P> {
        self.toks.iter().filter(|(_, k)| k.t == t && roles.contains(&k.r.as_str())).map(|(p, _)| *p).collect()
    }
}

fn root(p: &str) -> String {
    format!("{}/{p}", env!("CARGO_MANIFEST_DIR"))
}

fn layouts() -> Vec<Layout> {
    let mut out = vec![];
    let mut files: Vec<_> = std::fs::read_dir(root("../docs/layouts")).unwrap().flatten().map(|e| e.path()).collect();
    files.sort();
    for path in files.into_iter().filter(|p| p.extension().is_some_and(|e| e == "json")) {
        let json: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let mut toks = vec![];
        for (code, k) in json["keys"].as_object().into_iter().flatten() {
            for (layer, shift) in [("base", false), ("shift", true)] {
                for t in k[layer].as_array().into_iter().flatten() {
                    let tok = Tok { t: t["t"].as_str().unwrap_or("").into(), r: t["r"].as_str().unwrap_or("").into() };
                    toks.push(((Key::from_code(code), shift), tok));
                }
            }
        }
        toks.sort_by_key(|((_, s), _)| *s);
        out.push(Layout { id: json["id"].as_u64().unwrap() as u32, json, toks });
    }
    out
}

fn data() -> Arc<Data> {
    let read = |f: &str| std::fs::read_to_string(root(&format!("data/{f}"))).unwrap();
    let bin = |f: &str| std::fs::read(root(&format!("data/{f}"))).unwrap();
    Arc::new(
        Data::parse(&read("ko-words.tsv"), &read("en-words.txt"), &read("hanja.tsv"), &read("oecd.tsv"))
            .with_japanese(&read("ja-words.txt"))
            .with_kanji(&bin("kanji.bin"))
            .with_lm(&bin("ko-jamo-4gram.bin"))
            .with_search(&read("symbols.tsv"), &read("emoji.tsv"), &read("unicode-names.tsv")),
    )
}

/// What the presses show on a fresh engine with only this layout enabled, and the menu from entry 0.
fn screen(d: &Arc<Data>, id: u32, presses: &[P]) -> (String, Vec<String>) {
    let mut e = Engine::new(d.clone());
    e.pin_dictionary(None);
    let mut s = Settings::default();
    s.enabled = vec![id];
    e.set_settings(s);
    e.set_mode(Mode::Henkan);
    let mut committed = String::new();
    let mut out = bada::Output::default();
    for &(k, shift) in presses {
        out = e.key(k, if shift { Mods::SHIFT } else { Mods::NONE });
        committed.push_str(&out.commit);
    }
    let menu = out.menu.get(out.zero..).map(|m| m.to_vec()).unwrap_or_default();
    (format!("{committed}{}", out.preedit), menu)
}

fn pressed(presses: &[P]) -> Vec<Press> {
    presses.iter().map(|&(k, s)| Press::new(k, s)).collect()
}

fn composed(id: u32, presses: &[P]) -> String {
    compose_presses(id, &pressed(presses))
}

// ---------- Korean, composed (10–14, 16) ----------

const CHO: &str = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ";
const JUNG: &str = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ";
const JONG: &str = " ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ";
const VOWEL_JOINS: &[(char, char, char)] =
    &[('ㅘ', 'ㅗ', 'ㅏ'), ('ㅙ', 'ㅗ', 'ㅐ'), ('ㅚ', 'ㅗ', 'ㅣ'), ('ㅝ', 'ㅜ', 'ㅓ'), ('ㅞ', 'ㅜ', 'ㅔ'), ('ㅟ', 'ㅜ', 'ㅣ'), ('ㅢ', 'ㅡ', 'ㅣ')];
const CLUSTERS: &[(char, char, char)] = &[
    ('ㄳ', 'ㄱ', 'ㅅ'), ('ㄵ', 'ㄴ', 'ㅈ'), ('ㄶ', 'ㄴ', 'ㅎ'), ('ㄺ', 'ㄹ', 'ㄱ'), ('ㄻ', 'ㄹ', 'ㅁ'), ('ㄼ', 'ㄹ', 'ㅂ'),
    ('ㄽ', 'ㄹ', 'ㅅ'), ('ㄾ', 'ㄹ', 'ㅌ'), ('ㄿ', 'ㄹ', 'ㅍ'), ('ㅀ', 'ㄹ', 'ㅎ'), ('ㅄ', 'ㅂ', 'ㅅ'),
];
const TENSE: &[(char, char, char)] = &[('ㄲ', 'ㄱ', 'ㄱ'), ('ㄸ', 'ㄷ', 'ㄷ'), ('ㅃ', 'ㅂ', 'ㅂ'), ('ㅆ', 'ㅅ', 'ㅅ'), ('ㅉ', 'ㅈ', 'ㅈ')];

fn jamo(s: char) -> (char, char, Option<char>) {
    let n = s as u32 - 0xAC00;
    let f = JONG.chars().nth((n % 28) as usize).unwrap();
    (CHO.chars().nth((n / 588) as usize).unwrap(), JUNG.chars().nth((n % 588 / 28) as usize).unwrap(), (f != ' ').then_some(f))
}

fn split(j: char, table: &[(char, char, char)]) -> Option<(char, char)> {
    table.iter().find(|t| t.0 == j).map(|t| (t.1, t.2))
}

/// Ways to type one jamo in a slot: one key, or two that join.
fn slot(l: &Layout, j: char, roles: &[&str], joins: &[(char, char, char)]) -> Vec<Vec<P>> {
    let mut out: Vec<Vec<P>> = l.presses(&j.to_string(), roles).into_iter().map(|p| vec![p]).collect();
    let mut pairs: Vec<(char, char)> = split(j, joins).into_iter().collect();
    if l.rule("double_tense") {
        pairs.extend(split(j, TENSE));
    }
    for (a, b) in pairs {
        for pa in l.presses(&a.to_string(), roles) {
            for pb in l.presses(&b.to_string(), roles) {
                out.push(vec![pa, pb]);
            }
        }
    }
    out
}

/// Every way to type one syllable, fewest presses first (the robot's order).
fn syllable_plans(l: &Layout, s: char) -> Vec<Vec<P>> {
    let (i, v, f) = jamo(s);
    let mut inis = slot(l, i, &["i", "c"], &[]);
    if i == 'ㅇ' && l.rule("implied_ieung") {
        inis.push(vec![]);
    }
    let vowels = slot(l, v, &["v"], VOWEL_JOINS);
    let fins = match f {
        None => vec![vec![]],
        Some(f) => slot(l, f, &["f", "c"], CLUSTERS),
    };
    // Whole-syllable keys: ⇧ consonant with ㅏ (12), ⇧ vowel on ㅇ (11, 12).
    let mut heads: Vec<Vec<P>> = vec![];
    for a in inis {
        for b in &vowels {
            heads.push([a.clone(), b.clone()].concat());
        }
    }
    if v == 'ㅏ' {
        heads.extend(l.presses(&i.to_string(), &["a"]).into_iter().map(|p| vec![p]));
    }
    if i == 'ㅇ' {
        heads.extend(l.presses(&v.to_string(), &["n"]).into_iter().map(|p| vec![p]));
    }
    let mut out = vec![];
    for h in &heads {
        for t in &fins {
            out.push([h.clone(), t.clone()].concat());
        }
    }
    out.sort_by_key(|p| (p.len(), p.iter().filter(|x| x.1).count()));
    out.truncate(48);
    out
}

/// The ways to type one character on a layout, the robot's order, and where each one's share
/// ends among them (for drawing one).
struct Ways {
    plans: Vec<Vec<P>>,
    upto: Vec<f64>,
}

/// The ways to type each character of the Korean items: a syllable's plans, and for a jamo
/// outside a syllable (ㅋㅋ) its keys.
fn ways(l: &Layout, characters: &[char]) -> HashMap<char, Ways> {
    let list = parallel(characters, |&c| {
        let plans: Vec<Vec<P>> = if ('가'..='힣').contains(&c) {
            syllable_plans(l, c)
        } else {
            l.presses(&c.to_string(), &["c", "i", "f", "v"]).into_iter().map(|p| vec![p]).collect()
        };
        let shifts = |p: &Vec<P>| p.iter().filter(|x| x.1).count() as i32;
        let share = |p: &Vec<P>| LONGER.powi(p.len() as i32 - plans[0].len() as i32) * SHIFTED.powi(shifts(p) - shifts(&plans[0]));
        let total: f64 = plans.iter().map(share).sum();
        let mut sum = 0.0;
        let upto = plans.iter().map(|p| { sum += share(p) / total; sum }).collect();
        Ways { plans, upto }
    });
    characters.iter().copied().zip(list).collect()
}

/// The robot's plan for a word: syllable by syllable, the first plan that composes the word so far.
fn robot_plan(id: u32, ways: &HashMap<char, Ways>, word: &str) -> Option<Vec<P>> {
    let syl: Vec<char> = word.chars().collect();
    let mut beam: Vec<Vec<P>> = vec![vec![]];
    for k in 0..syl.len() {
        let want: String = syl[..=k].iter().collect();
        let mut next = vec![];
        'plans: for plan in &ways.get(&syl[k])?.plans {
            for b in &beam {
                let cand = [b.clone(), plan.clone()].concat();
                if composed(id, &cand) == want {
                    next.push(cand);
                    if next.len() >= BEAM {
                        break 'plans;
                    }
                }
            }
        }
        if next.is_empty() {
            return None;
        }
        beam = next;
    }
    beam.into_iter().next()
}

/// A number in 0..1 from `state` (xorshift), for drawing the same ways on every run.
fn random(state: &mut u64) -> f64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 11) as f64 / (1u64 << 53) as f64
}

/// Another way a person may type a word: for each syllable a way drawn by its share that
/// composes the word so far, with no Shift that changes nothing there (2-set's ⇧ㅏ, 11's ⇧ㅏ
/// where a plain ㅏ starts the syllable too). After a few draws that don't do: the first way
/// that composes it.
fn drawn_plan(id: u32, ways: &HashMap<char, Ways>, word: &str, state: &mut u64) -> Option<Vec<P>> {
    let syl: Vec<char> = word.chars().collect();
    let mut plan: Vec<P> = vec![];
    for k in 0..syl.len() {
        let want: String = syl[..=k].iter().collect();
        let w = ways.get(&syl[k])?;
        let composes = |way: &[P]| Some([&plan[..], way].concat()).filter(|c| composed(id, c) == want);
        let needless = |way: &Vec<P>| {
            (0..way.len()).filter(|&i| way[i].1).any(|i| {
                let mut plain = way.clone();
                plain[i].1 = false;
                composes(&plain).is_some()
            })
        };
        let with = |way: &Vec<P>| composes(way).filter(|_| !needless(way));
        let mut next = None;
        for _ in 0..4 {
            let at = random(state);
            let i = w.upto.partition_point(|&u| u <= at).min(w.plans.len().saturating_sub(1));
            next = w.plans.get(i).and_then(with);
            if next.is_some() {
                break;
            }
        }
        plan = next.or_else(|| w.plans.iter().find_map(|way| composes(way)))?;
    }
    Some(plan)
}

// ---------- spelled: English, Korean romanization, kana, romaji ----------

fn spell(l: &Layout, letters: &str) -> Option<Vec<P>> {
    letters.chars().map(|c| l.toks.iter().find(|(_, t)| t.t.chars().eq([c])).map(|(p, _)| *p)).collect()
}

/// Layouts 15 and 18: the word in the spelling table, syllables run together, and split by `break`.
fn roman_plans(l: &Layout, word: &str) -> Option<Vec<Vec<P>>> {
    let t = &l.json["spelling"];
    let get = |table: &str, j: char| t[table][j.to_string()].as_str().map(String::from);
    let mut syls = vec![];
    for s in word.chars() {
        if !('가'..='힣').contains(&s) {
            return None;
        }
        let (i, v, f) = jamo(s);
        let mut x = get("onsets", i)? + &get("vowels", v)?;
        if let Some(f) = f {
            x += &get("codas", f)?;
        }
        syls.push(x);
    }
    let br = t["break"].as_str().unwrap_or("-");
    Some([syls.concat(), syls.join(br)].iter().filter_map(|s| spell(l, s)).collect())
}

/// Layout 20: each kana on its key, a voiced one as its plain kana and ゛ (゜); 、 and 。 on theirs.
fn kana_plan(l: &Layout, reading: &str) -> Option<Vec<P>> {
    let mut out = vec![];
    for c in reading.chars() {
        if let Some(p) = l.presses(&c.to_string(), &["k", "x"]).first() {
            out.push(*p);
            continue;
        }
        let n = c as u32;
        let (base, mark) = if c == 'ゔ' {
            ('う', "゛")
        } else if let Some(b) = char::from_u32(n - 1).filter(|_| "がぎぐげござじずぜぞだぢづでどばびぶべぼ".contains(c)) {
            (b, "゛")
        } else if let Some(b) = char::from_u32(n - 2).filter(|_| "ぱぴぷぺぽ".contains(c)) {
            (b, "゜")
        } else {
            return None;
        };
        out.push(*l.presses(&base.to_string(), &["k"]).first()?);
        out.push(*l.presses(mark, &["k"]).first()?);
    }
    Some(out)
}

/// Layout 21: the reading in the spelling table, two kana at once where the table has them; っ
/// doubles the next consonant; ん is nn, or n' before a vowel, y or n; 、 and 。 are the comma and
/// the full stop. `kunrei`: its spellings where they differ. `one_n`: ん as one n wherever the
/// next letter lets it be read so (before a consonant but y, and at the end).
fn romaji_plan(l: &Layout, reading: &str, (kunrei, one_n): (bool, bool)) -> Option<Vec<P>> {
    let t = &l.json["spelling"];
    let spelled = |kana: &str| {
        let other = KUNREI.iter().find(|k| kunrei && k.0 == kana).map(|k| k.1.to_string());
        other.or_else(|| t[kana].as_str().map(String::from))
    };
    let kana: Vec<char> = reading.chars().collect();
    let mut letters = String::new();
    let mut i = 0;
    let mut sokuon = false;
    while i < kana.len() {
        if kana[i] == '、' || kana[i] == '。' {
            letters.push(if kana[i] == '、' { ',' } else { '.' });
            i += 1;
            continue;
        }
        if kana[i] == 'っ' {
            if sokuon {
                letters += t["っ"].as_str()?;
            }
            sokuon = true;
            i += 1;
            continue;
        }
        let two: String = kana[i..(i + 2).min(kana.len())].iter().collect();
        let (mut r, n) = match spelled(&two) {
            Some(r) if two.chars().count() == 2 => (r, 2),
            _ => (spelled(&kana[i].to_string())?, 1),
        };
        if kana[i] == 'ん' {
            let next = kana.get(i + 1).and_then(|k| spelled(&k.to_string())).and_then(|s| s.chars().next());
            if next.is_some_and(|c| "aiueoy".contains(c) || (c == 'n' && !one_n)) {
                r = t["ん'"].as_str()?.to_string();
            } else if one_n {
                r = "n".to_string();
            }
        }
        if sokuon {
            let c = r.chars().next()?;
            if "aiueon-".contains(c) {
                letters += t["っ"].as_str()?;
            } else {
                letters.push(c);
            }
            sokuon = false;
        }
        letters += &r;
        i += n;
    }
    if sokuon {
        letters += t["っ"].as_str()?;
    }
    spell(l, &letters)
}

// ---------- what is typed ----------

/// One run of keys for one model: an item typed one way, with that way's part of the item's weight
/// (then, weighed, its weight).
struct Run {
    item: usize,
    model: usize,
    symbols: Vec<u8>,
    weight: f64,
    test: bool,
}

/// `f` over the items on every core, the results in order.
fn parallel<I: Sync, T: Send, F: Fn(&I) -> T + Sync>(items: &[I], f: F) -> Vec<T> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let size = items.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = items.chunks(size).map(|part| s.spawn(|| part.iter().map(&f).collect::<Vec<T>>())).collect();
        handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
}

fn fnv(word: &str) -> u64 {
    word.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

/// A listed item: (language, source, kind), held out or not, the text and its count.
struct Item {
    group: (String, String, String),
    test: bool,
    text: String,
    count: f64,
}

fn items() -> Vec<Item> {
    let path = root("tools/cache/keymodel/lists.tsv");
    let lists = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{path}: run engine/tools/build_keymodel.py"));
    let mut out = vec![];
    for line in lists.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let [lang, source, kind, split, text, n] = f[..] else { continue };
        out.push(Item { group: (lang.into(), source.into(), kind.into()), test: split == "test", text: text.into(), count: n.parse().unwrap() });
    }
    let words = std::fs::read_to_string(root("data/ko-words.tsv")).unwrap();
    for line in words.lines().filter(|l| !l.starts_with('#')) {
        let Some((word, n)) = line.split_once('\t') else { continue };
        let test = fnv(word) % HELD == 0;
        out.push(Item { group: ("ko".into(), "words".into(), "word".into()), test, text: word.into(), count: n.trim().parse().unwrap() });
    }
    out
}

/// Every item typed on the layouts of its language: the runs of keys, each with its part of the
/// item's weight.
fn runs(all: &[Layout], d: &Arc<Data>, items: &[Item]) -> Vec<Run> {
    let layout = |id: u32| all.iter().find(|l| l.id == id).unwrap();
    let model = |id: u32| LAYOUTS.iter().position(|&x| x == id).unwrap();
    // Korean words by rank: the commonest are drawn more ways, and checked on the engine for 15 and 18.
    let mut korean: Vec<(&str, f64)> = items.iter().filter(|i| i.group.0 == "ko").map(|i| (i.text.as_str(), i.count)).collect();
    korean.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(b.0)));
    let rank: HashMap<&str, usize> = korean.iter().enumerate().rev().map(|(n, w)| (w.0, n)).collect();
    let mut characters: Vec<char> = korean.iter().flat_map(|w| w.0.chars()).collect();
    characters.sort_unstable();
    characters.dedup();
    let composers: Vec<(u32, HashMap<char, Ways>)> = [10, 11, 12, 13, 14, 16].iter().map(|&id| (id, ways(layout(id), &characters))).collect();
    // What a spelling shows on 15 and 18 before any ranking: the reading the letters spell.
    let spelled = |id: u32, plan: &[P]| {
        if id == 15 { bada::bada_roman::compose_presses(&pressed(plan)) } else { bada::korean_roman::compose_presses(&pressed(plan)) }
    };
    // Each Korean layout's key for each of MARKS: the first press (unshifted first) after which 가
    // typed on the layout shows 가 and the mark.
    let presses: Vec<P> = [false, true].iter().flat_map(|&s| Key::ALL.iter().filter(|k| k.is_kana_space()).map(move |&k| (k, s))).collect();
    let marks: HashMap<u32, Vec<(P, f64)>> = [10, 11, 12, 13, 14, 15, 16, 18]
        .iter()
        .map(|&id| {
            let ga = match composers.iter().find(|c| c.0 == id) {
                Some((_, ways)) => robot_plan(id, ways, "가"),
                None => roman_plans(layout(id), "가").and_then(|p| p.into_iter().next()),
            };
            let key = |mark: char| presses.iter().copied().find(|&p| ga.as_ref().is_some_and(|ga| screen(d, id, &[&ga[..], &[p]].concat()).0 == format!("가{mark}")));
            (id, MARKS.iter().filter_map(|&(mark, share)| Some((key(mark)?, share))).collect())
        })
        .collect();
    let typed: Vec<Vec<Run>> = parallel(&(0..items.len()).collect::<Vec<_>>(), |&n| {
        let item = &items[n];
        let mut out: Vec<Run> = vec![];
        // The same keys twice for one model are one run.
        let mut add = |id: u32, presses: Option<Vec<P>>, part: f64| {
            let symbols: Option<Vec<u8>> = presses.as_deref().and_then(|p| p.iter().map(|&(k, s)| symbol(Press::new(k, s))).collect());
            let Some(symbols) = symbols.filter(|s| !s.is_empty()) else { return };
            match out.iter_mut().find(|r| r.model == model(id) && r.symbols == symbols) {
                Some(run) => run.weight += part,
                None => out.push(Run { item: n, model: model(id), symbols, weight: part, test: item.test }),
            }
        };
        match item.group.0.as_str() {
            "en" => add(0, spell(layout(0), &item.text), 1.0),
            "ja" => {
                add(20, kana_plan(layout(20), &item.text), 1.0);
                // The robot's spelling as it is; the others where the engine reads them as the kana.
                let l = layout(21);
                for (style, share) in ROMAJI {
                    let plan = romaji_plan(l, &item.text, style);
                    let reads = |p: &Vec<P>| bada::japanese::romaji_compose(&pressed(p)) == item.text;
                    let plan = if style == ROMAJI[0].0 { plan } else { plan.filter(reads).or_else(|| romaji_plan(l, &item.text, ROMAJI[0].0)) };
                    add(21, plan, share);
                }
            }
            _ => {
                let rank = rank[item.text.as_str()];
                for (id, ways) in &composers {
                    let Some(robot) = robot_plan(*id, ways, &item.text) else { continue };
                    let drawn = DRAWN.iter().find(|d| rank < d.0).map_or(1, |d| d.1);
                    let mut state = fnv(&item.text) ^ (*id as u64).wrapping_mul(0x9E3779B97F4A7C15) | 1;
                    for _ in 0..drawn {
                        let plan = drawn_plan(*id, ways, &item.text, &mut state).unwrap_or_else(|| robot.clone());
                        add(*id, Some(plan), (1.0 - ROBOT) / drawn as f64);
                    }
                    for &(mark, share) in marks[id].iter().filter(|_| rank < MARKED) {
                        add(*id, Some([&robot[..], &[mark]].concat()), share);
                    }
                    add(*id, Some(robot), ROBOT);
                }
                for id in [15, 18] {
                    let l = layout(id);
                    let Some(plans) = roman_plans(l, &item.text) else { continue };
                    // The robot's pick: the first spelling that shows the word, else the first
                    // that offers it in the first rows. On the engine for the commonest words;
                    // for the rest, by the reading the letters spell.
                    let row = |p: &Vec<P>| {
                        let (shown, menu) = screen(d, id, p);
                        if shown == item.text { Some(0) } else { menu.iter().take(TOP).position(|m| *m == item.text) }
                    };
                    let best = if rank < CHECKED {
                        let rows: Vec<Option<usize>> = plans.iter().map(row).collect();
                        rows.iter().position(|r| *r == Some(0)).or_else(|| rows.iter().position(Option::is_some)).unwrap_or(0)
                    } else {
                        plans.iter().position(|p| spelled(id, p) == item.text).unwrap_or(0)
                    };
                    for &(mark, share) in marks[&id].iter().filter(|_| rank < MARKED) {
                        add(id, Some([&plans[best][..], &[mark]].concat()), share);
                    }
                    for (n, plan) in plans.into_iter().enumerate() {
                        add(id, Some(plan), SPELLINGS[(n != best) as usize]);
                    }
                }
            }
        }
        out
    });
    typed.into_iter().flatten().collect()
}

/// Each item's weight: its count's share of its source's part of the mass (the held-out items
/// weighed as the others, by the totals without them).
fn weights(items: &[Item]) -> Vec<f64> {
    let mut totals: HashMap<&(String, String, String), f64> = HashMap::new();
    for i in items.iter().filter(|i| !i.test) {
        *totals.entry(&i.group).or_default() += i.count;
    }
    items
        .iter()
        .map(|item| {
            let (lang, source, kind) = (item.group.0.as_str(), item.group.1.as_str(), item.group.2.as_str());
            let share = SHARES.iter().find(|s| (s.0, s.1, s.2) == (lang, source, kind)).map_or(0.0, |s| s.3);
            item.count * share * MASS / totals.get(&item.group).copied().unwrap_or(1.0)
        })
        .collect()
}

// ---------- counting and smoothing ----------

/// One history's continuations: (symbol, count), ascending.
type Continuations = Vec<(u8, f64)>;

/// The counts of one model: per n-gram length, each history's continuations.
struct Counts {
    unigrams: Vec<f64>,
    /// `levels[n]`: histories of n + 1 symbols.
    levels: Vec<BTreeMap<u32, Continuations>>,
}

fn count(runs: &[Run], model: usize, with_test: bool) -> Counts {
    let mut unigrams = vec![0.0; A];
    let mut maps: Vec<HashMap<(u32, u8), f64>> = vec![HashMap::new(); ORDER - 1];
    for run in runs.iter().filter(|r| r.model == model && (with_test || !r.test)) {
        let mut seq = vec![BOUNDARY];
        seq.extend_from_slice(&run.symbols);
        seq.push(BOUNDARY);
        for i in 1..seq.len() {
            unigrams[seq[i] as usize] += run.weight;
            for n in 1..ORDER.min(i + 1) {
                *maps[n - 1].entry((history_key(&seq[i - n..i]), seq[i])).or_default() += run.weight;
            }
        }
    }
    let levels = maps
        .into_iter()
        .map(|m| {
            let mut level: BTreeMap<u32, Continuations> = BTreeMap::new();
            for ((h, s), c) in m {
                level.entry(h).or_default().push((s, c));
            }
            level.values_mut().for_each(|v| v.sort_by_key(|x| x.0));
            level
        })
        .collect();
    Counts { unigrams, levels }
}

fn digits(mut key: u32, n: usize) -> Vec<u8> {
    let mut out = vec![0u8; n];
    for d in out.iter_mut().rev() {
        *d = (key % A as u32) as u8;
        key /= A as u32;
    }
    out
}

/// A history's listed continuations: (symbol, probability, gain). The gain is what listing the
/// n-gram adds to the likelihood of the counts, in bits per item of MASS.
type Listed = Vec<(u8, f64, f64)>;

/// A model as probabilities: each listed n-gram's, and each history's backoff weight.
struct Smoothed {
    unigrams: Vec<f64>,
    /// Per level, history → (backoff weight, its listed continuations).
    levels: Vec<BTreeMap<u32, (f64, Listed)>>,
}

impl Smoothed {
    /// P(next | history) from the listed n-grams and the backoff weights, as the engine reads them.
    fn probability(&self, history: &[u8], next: u8) -> f64 {
        let mut weight = 1.0;
        for start in 0..history.len() {
            let h = &history[start..];
            let Some((backoff, list)) = self.levels[h.len() - 1].get(&history_key(h)) else { continue };
            if let Ok(i) = list.binary_search_by_key(&next, |x| x.0) {
                return weight * list[i].1;
            }
            weight *= backoff;
        }
        weight * self.unigrams[next as usize]
    }
}

/// Witten–Bell, interpolated: P(w | h) = (c(h w) + T(h) P(w | h')) / (N(h) + T(h)), with T the
/// number of different symbols seen after h.
fn smooth(counts: &Counts) -> Smoothed {
    let total: f64 = counts.unigrams.iter().sum();
    let types = counts.unigrams.iter().filter(|&&c| c > 0.0).count() as f64;
    let unigrams: Vec<f64> = counts.unigrams.iter().map(|&c| (c + types / A as f64) / (total + types)).collect();
    let mut model = Smoothed { unigrams, levels: vec![] };
    for (n, level) in counts.levels.iter().enumerate() {
        let mut out = BTreeMap::new();
        for (&key, list) in level {
            let h = digits(key, n + 1);
            let seen: f64 = list.iter().map(|x| x.1).sum();
            let types = list.len() as f64;
            let backoff = types / (seen + types);
            let listed = list
                .iter()
                .map(|&(s, c)| {
                    let lower = model.probability(&h[1..], s);
                    let p = (c + types * lower) / (seen + types);
                    (s, p, c / MASS * (p / (backoff * lower)).log2())
                })
                .collect();
            out.insert(key, (backoff, listed));
        }
        model.levels.push(out);
    }
    model
}

/// A history's n-grams worth listing at `threshold`: those whose gain reaches it, if together they
/// are worth the history too (it takes the room of three n-grams).
fn kept(list: &Listed, threshold: f64) -> Listed {
    let kept: Listed = list.iter().copied().filter(|x| x.2 >= threshold).collect();
    if kept.iter().map(|x| x.2).sum::<f64>() < threshold * (3 + kept.len()) as f64 { vec![] } else { kept }
}

/// The model with the n-grams of gain under `threshold` left out, and each history's backoff
/// weight set again so its probabilities sum to one.
fn prune(full: &Smoothed, threshold: f64) -> Smoothed {
    let mut model = Smoothed { unigrams: full.unigrams.clone(), levels: vec![] };
    for (n, level) in full.levels.iter().enumerate() {
        let mut out = BTreeMap::new();
        for (&key, (_, list)) in level {
            let kept = kept(list, threshold);
            if kept.is_empty() {
                continue;
            }
            let h = digits(key, n + 1);
            let listed: f64 = kept.iter().map(|x| x.1).sum();
            let lower: f64 = kept.iter().map(|x| model.probability(&h[1..], x.0)).sum();
            let backoff = ((1.0 - listed) / (1.0 - lower).max(1e-12)).clamp(1e-9, 1.0);
            out.insert(key, (backoff, kept));
        }
        model.levels.push(out);
    }
    model
}

/// The bytes a model takes pruned at `threshold` (as `tables` writes it).
fn size(full: &Smoothed, threshold: f64) -> usize {
    let level = |level: &BTreeMap<u32, (f64, Listed)>| -> usize {
        8 + level.values().map(|v| kept(&v.1, threshold).len()).filter(|&n| n > 0).map(|n| 6 + 2 * n).sum::<usize>()
    };
    A + full.levels.iter().map(level).sum::<usize>()
}

fn byte(p: f64) -> u8 {
    (-16.0 * p.log2()).round().clamp(0.0, 255.0) as u8
}

/// One model's tables (src/keymodel.rs has the format).
fn tables(model: &Smoothed) -> Vec<u8> {
    let mut out: Vec<u8> = model.unigrams.iter().map(|&p| byte(p)).collect();
    for level in &model.levels {
        let continuations: usize = level.values().map(|v| v.1.len()).sum();
        out.extend((level.len() as u32).to_le_bytes());
        out.extend((continuations as u32).to_le_bytes());
        level.keys().for_each(|k| out.extend(k.to_le_bytes()));
        out.extend(level.values().map(|v| byte(v.0)));
        out.extend(level.values().map(|v| v.1.len() as u8));
        out.extend(level.values().flat_map(|v| v.1.iter().map(|x| x.0)));
        out.extend(level.values().flat_map(|v| v.1.iter().map(|x| byte(x.1))));
    }
    out
}

fn file(models: &[Smoothed]) -> Vec<u8> {
    let tables: Vec<Vec<u8>> = models.iter().map(tables).collect();
    let mut out = b"BKM1".to_vec();
    for v in [1u32, ORDER as u32, A as u32, models.len() as u32] {
        out.extend(v.to_le_bytes());
    }
    for (id, t) in LAYOUTS.iter().zip(&tables) {
        out.extend(id.to_le_bytes());
        out.extend((t.len() as u32).to_le_bytes());
    }
    tables.iter().for_each(|t| out.extend(t));
    out
}

/// The models pruned with one threshold for all, the lowest that keeps the file within `bytes`.
fn fit(full: &[Smoothed], bytes: usize) -> (Vec<Smoothed>, f64) {
    let header = 20 + 8 * full.len();
    let total = |t: f64| header + parallel(full, |m| size(m, t)).iter().sum::<usize>();
    let (mut low, mut high) = (0.0f64, 1e-3f64);
    if total(low) > bytes {
        while high / low.max(1e-30) > 1.01 {
            let mid = if low == 0.0 { high * 1e-3 } else { (low * high).sqrt() };
            if total(mid) <= bytes { high = mid } else { low = mid }
            if mid < 1e-30 {
                break;
            }
        }
        low = high;
    }
    (parallel(full, |m| prune(m, low)), low)
}

// ---------- the report ----------

/// How the held-out runs of each layout are read: against each layout of another language, the
/// share (by weight) its own model explains better as a whole word; and against all of them.
fn held_out(models: &KeyModels, runs: &[Run]) {
    let language = |id: u32| id / 10;
    eprintln!("held out: read as its own layout against a layout of another language (% of weight), whole words");
    eprint!("{:>4}", "");
    LAYOUTS.iter().for_each(|id| eprint!("{id:>7}"));
    eprintln!("{:>8}{:>10}", "all", "bits/key");
    for (m, &id) in LAYOUTS.iter().enumerate() {
        let mut right = vec![0.0; LAYOUTS.len() + 1];
        let (mut total, mut bits, mut keys) = (0.0, 0.0, 0.0);
        for run in runs.iter().filter(|r| r.test && r.model == m) {
            let scores: Vec<f64> = LAYOUTS.iter().map(|&x| models.score(x, &run.symbols).map_or(f64::MIN, |s| s.0 + s.1)).collect();
            let mut all = true;
            for (o, &other) in LAYOUTS.iter().enumerate() {
                if language(other) != language(id) {
                    let won = scores[m] > scores[o];
                    right[o] += run.weight * won as u8 as f64;
                    all &= won;
                }
            }
            right[LAYOUTS.len()] += run.weight * all as u8 as f64;
            total += run.weight;
            bits -= run.weight * scores[m] / std::f64::consts::LN_2;
            keys += run.weight * (run.symbols.len() + 1) as f64;
        }
        eprint!("{id:>4}");
        for (o, &other) in LAYOUTS.iter().enumerate() {
            if language(other) == language(id) { eprint!("{:>7}", "—") } else { eprint!("{:>7.2}", 100.0 * right[o] / total) }
        }
        eprintln!("{:>8.2}{:>10.3}", 100.0 * right[LAYOUTS.len()] / total, bits / keys);
    }
}

fn main() {
    let started = std::time::Instant::now();
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned());
    let out = flag("--out").unwrap_or_else(|| root("data/key-models.bin"));
    let bytes: usize = flag("--size").map_or(SIZE, |s| s.parse().unwrap());
    let all = layouts();
    let d = data();
    let items = items();
    let mut runs = runs(&all, &d, &items);
    let weights = weights(&items);
    runs.iter_mut().for_each(|r| r.weight *= weights[r.item]);
    runs.retain(|r| r.weight > 0.0);
    eprintln!("{} items typed as {} runs of keys ({:.0} s)", items.len(), runs.len(), started.elapsed().as_secs_f64());

    // Without the held-out items first, for the report; then with everything, for the file.
    for with_test in [false, true] {
        let full: Vec<Smoothed> = parallel(&(0..LAYOUTS.len()).collect::<Vec<_>>(), |&m| smooth(&count(&runs, m, with_test)));
        let (models, threshold) = fit(&full, bytes);
        let written = file(&models);
        let parsed = KeyModels::parse(&written).expect("the file parses");
        if !with_test {
            held_out(&parsed, &runs);
            continue;
        }
        for ((id, f), m) in LAYOUTS.iter().zip(&full).zip(&models) {
            let n = |s: &Smoothed| s.levels.iter().map(|l| l.values().map(|v| v.1.len()).sum::<usize>()).collect::<Vec<_>>();
            eprintln!("{id:>3}: n-grams {:?} of {:?}, {} bytes", n(m), n(f), tables(m).len());
        }
        std::fs::write(&out, &written).unwrap();
        eprintln!("{out}: {} bytes (threshold {threshold:.3e} bits per item), {:.0} s", written.len(), started.elapsed().as_secs_f64());
    }
}
