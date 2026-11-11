//! The typing robot (docs/robot.md): for each layout in docs/layouts/*.json and each language it
//! declares, type the corpora in engine/tests/data/robot/ the way a person would, planning the keys
//! from the layout's JSON alone and reading the screen (the engine's output) as they go.
//!
//! A word fails if it can't be typed, if typing it needs a commit in the middle of the word (the
//! word only comes out right as two pieces), or, for layouts that convert (15, 18, 19, 20, 21), if
//! the wanted text isn't among the first `TOP` rows.
//!
//! Each layout claims a reliability for each language it declares (`reliability` in its JSON), and
//! must earn it: `perfect`, every item of every corpus; `production`, every one of the language's
//! most common words (`*-common.*`) and `PRODUCTION_RATE` of each other corpus; `experimental`,
//! no bar.
//!
//!     cargo test --release --test robot -- --nocapture
//!
//! ROBOT_LIMIT=n caps the distinct items per corpus (default: all). The report, with failing
//! examples: target/robot-report.md.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::sync::Arc;

use bada::engine::Mode;
use bada::{Data, Engine, Key, Mods, Settings};
use serde_json::Value;

/// Rows of a converting layout's menu the wanted text must be in.
const TOP: usize = 5;
/// `production`: the share of each corpus (other than the common words) that must be typed.
const PRODUCTION_RATE: f64 = 0.80;
/// Reliabilities a layout can claim for a language, weakest first.
const LEVELS: [&str; 3] = ["experimental", "production", "perfect"];
/// Plans tried per syllable, kept per step.
const BEAM: usize = 4;

// ---------- data ----------

fn root(p: &str) -> String {
    format!("{}/{p}", env!("CARGO_MANIFEST_DIR"))
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

#[derive(Clone, Debug)]
struct Tok {
    t: String,
    r: String,
}

type Press = (Key, bool);

struct Layout {
    id: u32,
    file: String,
    languages: Vec<String>,
    /// Language → claimed reliability (one of `LEVELS`).
    reliability: BTreeMap<String, String>,
    input: String,
    json: Value,
    /// Every token with the press that types it.
    toks: Vec<(Press, Tok)>,
}

impl Layout {
    fn rule(&self, name: &str) -> bool {
        self.json["rules"][name].as_bool().unwrap_or(false)
    }
    fn presses(&self, t: &str, roles: &[&str]) -> Vec<Press> {
        self.toks.iter().filter(|(_, k)| k.t == t && roles.contains(&k.r.as_str())).map(|(p, _)| *p).collect()
    }
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
        // Unshifted first: a person reaches for the plain key before ⇧.
        toks.sort_by_key(|((_, s), _)| *s);
        out.push(Layout {
            id: json["id"].as_u64().unwrap() as u32,
            file: path.file_name().unwrap().to_string_lossy().into(),
            languages: json["languages"].as_array().into_iter().flatten().filter_map(|l| l.as_str().map(String::from)).collect(),
            reliability: json["reliability"].as_object().into_iter().flatten()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect(),
            input: json["input"].as_str().unwrap_or("").into(),
            json,
            toks,
        });
    }
    out
}

/// One thing to type: the text wanted, and for Japanese the reading typed.
#[derive(Clone)]
struct Item {
    text: String,
    reading: String,
}

struct Corpus {
    name: String,
    /// The language's most common words (`*-common.*`): `production` must type all of them.
    common: bool,
    items: Vec<Item>,
    /// Characters in the corpus that are no word of the language (punctuation, digits in Korean …).
    skipped: usize,
}

fn corpora(language: &str) -> Vec<Corpus> {
    let prefix = match language {
        "english" => "en-",
        "korean" => "ko-",
        "japanese_kana" | "japanese_romaji" => "ja-",
        _ => return vec![],
    };
    let limit: usize = std::env::var("ROBOT_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
    let mut files: Vec<_> = std::fs::read_dir(root("tests/data/robot")).unwrap().flatten().map(|e| e.path()).collect();
    files.sort();
    let mut out = vec![];
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if !name.starts_with(prefix) {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let mut seen = HashSet::new();
        let mut items = vec![];
        let mut skipped = 0;
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            if prefix == "ja-" {
                let Some((surface, reading)) = line.split_once('\t') else { continue };
                if seen.insert(surface.to_string()) {
                    items.push(Item { text: surface.into(), reading: reading.into() });
                }
                continue;
            }
            let word_char = |c: char| if prefix == "en-" { c.is_ascii_alphanumeric() || c == '\'' } else { ('가'..='힣').contains(&c) };
            let mut word = String::new();
            for c in line.chars().chain([' ']) {
                if word_char(c) {
                    word.push(c);
                    continue;
                }
                if !c.is_whitespace() {
                    skipped += 1;
                }
                let w = std::mem::take(&mut word);
                let w = w.trim_matches('\'').to_string();
                if !w.is_empty() && seen.insert(w.clone()) {
                    items.push(Item { text: w.clone(), reading: w });
                }
            }
        }
        items.truncate(limit);
        out.push(Corpus { common: name.contains("-common."), name, items, skipped });
    }
    out
}

// ---------- the screen ----------

/// What the screen shows after the presses on a fresh engine: everything committed and the
/// marked text, and the menu's rows from entry 0.
fn screen(d: &Arc<Data>, id: u32, presses: &[Press]) -> (String, Vec<String>) {
    let mut e = Engine::new(d.clone());
    e.pin_dictionary(None);
    let mut s = Settings::default();
    s.enabled = vec![id];
    e.set_settings(s);
    e.set_mode(Mode::Henkan);
    let mut shown = String::new();
    let mut out = bada::Output::default();
    for &(k, shift) in presses {
        out = e.key(k, if shift { Mods::SHIFT } else { Mods::NONE });
        shown.push_str(&out.commit);
    }
    shown.push_str(&out.preedit);
    let menu = out.menu.get(out.zero..).map(|m| m.to_vec()).unwrap_or_default();
    (shown, menu)
}

// ---------- Korean, composed ----------

const CHO: &str = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ";
const JUNG: &str = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ";
const JONG: &str = " ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ";

fn jamo(s: char) -> (char, char, Option<char>) {
    let n = s as u32 - 0xAC00;
    let f = JONG.chars().nth((n % 28) as usize).unwrap();
    (CHO.chars().nth((n / 588) as usize).unwrap(), JUNG.chars().nth((n % 588 / 28) as usize).unwrap(), (f != ' ').then_some(f))
}

fn split(j: char, table: &[(char, char, char)]) -> Option<(char, char)> {
    table.iter().find(|t| t.0 == j).map(|t| (t.1, t.2))
}
const VOWEL_JOINS: &[(char, char, char)] =
    &[('ㅘ', 'ㅗ', 'ㅏ'), ('ㅙ', 'ㅗ', 'ㅐ'), ('ㅚ', 'ㅗ', 'ㅣ'), ('ㅝ', 'ㅜ', 'ㅓ'), ('ㅞ', 'ㅜ', 'ㅔ'), ('ㅟ', 'ㅜ', 'ㅣ'), ('ㅢ', 'ㅡ', 'ㅣ')];
const CLUSTERS: &[(char, char, char)] = &[
    ('ㄳ', 'ㄱ', 'ㅅ'), ('ㄵ', 'ㄴ', 'ㅈ'), ('ㄶ', 'ㄴ', 'ㅎ'), ('ㄺ', 'ㄹ', 'ㄱ'), ('ㄻ', 'ㄹ', 'ㅁ'), ('ㄼ', 'ㄹ', 'ㅂ'),
    ('ㄽ', 'ㄹ', 'ㅅ'), ('ㄾ', 'ㄹ', 'ㅌ'), ('ㄿ', 'ㄹ', 'ㅍ'), ('ㅀ', 'ㄹ', 'ㅎ'), ('ㅄ', 'ㅂ', 'ㅅ'),
];
const TENSE: &[(char, char, char)] = &[('ㄲ', 'ㄱ', 'ㄱ'), ('ㄸ', 'ㄷ', 'ㄷ'), ('ㅃ', 'ㅂ', 'ㅂ'), ('ㅆ', 'ㅅ', 'ㅅ'), ('ㅉ', 'ㅈ', 'ㅈ')];

/// Ways to type one jamo in a slot: one key, or two that join (ㅗ ㅏ, ㄹ ㄱ, and ㄱ ㄱ where the
/// layout doubles).
fn slot(l: &Layout, j: char, roles: &[&str], joins: &[(char, char, char)]) -> Vec<Vec<Press>> {
    let mut out: Vec<Vec<Press>> = l.presses(&j.to_string(), roles).into_iter().map(|p| vec![p]).collect();
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

/// Every way to type one syllable, fewest presses first.
fn syllable_plans(l: &Layout, s: char) -> Vec<Vec<Press>> {
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
    let mut heads: Vec<Vec<Press>> = vec![];
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

#[derive(Debug, PartialEq)]
enum Fail {
    /// No plan types it: the syllable where every plan went wrong, and what showed.
    Untypeable(usize, String),
    /// It types only with a commit before syllable n.
    CommitMidWord(usize),
    /// Converting layouts: not among the first `TOP` rows (what row 0 showed).
    NotOffered(String),
}

/// Types a Korean word syllable by syllable, keeping the plans whose screen shows the word so far.
fn type_composed(d: &Arc<Data>, l: &Layout, word: &str) -> Result<usize, Fail> {
    let syl: Vec<char> = word.chars().collect();
    let mut beam: Vec<Vec<Press>> = vec![vec![]];
    for k in 0..syl.len() {
        let want: String = syl[..=k].iter().collect();
        let mut next = vec![];
        let mut last = String::new();
        'plans: for plan in syllable_plans(l, syl[k]) {
            for b in &beam {
                let cand = [b.clone(), plan.clone()].concat();
                let (shown, _) = screen(d, l.id, &cand);
                if shown == want {
                    next.push(cand);
                    if next.len() >= BEAM {
                        break 'plans;
                    }
                } else {
                    last = shown;
                }
            }
        }
        if next.is_empty() {
            // Would it come out with a commit here? (The rest typed on its own.)
            if k > 0 && type_composed(d, l, &syl[k..].iter().collect::<String>()).is_ok() {
                return Err(Fail::CommitMidWord(k));
            }
            return Err(Fail::Untypeable(k, last));
        }
        beam = next;
    }
    Ok(0)
}

// ---------- Korean, transcribed in kana (19) ----------

/// Layout 19's `transcription` section, read once: what ゛ ゜ っ do to an initial, built from the
/// tenseness series, and the readings the menu offers.
struct Trans {
    /// っ (a modifier typed before the kana): initial → its tense form.
    raise: BTreeMap<char, char>,
    /// Modifiers typed after the kana (゛ ゜): their press and initial → initial.
    after: Vec<(Press, BTreeMap<char, char>)>,
    sokuon: Option<Press>,
    lengthen: Option<(Press, BTreeMap<char, char>)>,
    vowel_readings: BTreeMap<char, Vec<char>>,
    final_readings: BTreeMap<char, Vec<String>>,
}

fn first_char(s: &str) -> char {
    s.chars().next().unwrap_or(' ')
}

fn trans(l: &Layout) -> Trans {
    let t = &l.json["transcription"];
    let series: Vec<&Value> = t["tenseness"].as_array().into_iter().flatten().collect();
    let level = |to: &str| -> BTreeMap<char, char> {
        let mut m = BTreeMap::new();
        for s in &series {
            let Some(target) = s[to].as_str() else { continue };
            for from in ["lax", "aspirated", "tense"] {
                if let Some(x) = s[from].as_str().filter(|x| *x != target) {
                    // Down to lax only from aspirated: a tense initial isn't lowered.
                    if to == "lax" && from != "aspirated" {
                        continue;
                    }
                    m.insert(first_char(x), first_char(target));
                }
            }
        }
        m
    };
    let mut out = Trans { raise: BTreeMap::new(), after: vec![], sokuon: None, lengthen: None, vowel_readings: BTreeMap::new(), final_readings: BTreeMap::new() };
    for (token, m) in t["modifiers"].as_object().into_iter().flatten() {
        let Some(press) = press_for(l, token) else { continue };
        let mut map = m["to"].as_str().map(level).unwrap_or_default();
        for (a, b) in m["also"].as_object().into_iter().flatten() {
            map.insert(first_char(a), first_char(b.as_str().unwrap_or("")));
        }
        if m["position"] == "before" {
            out.raise = map;
            out.sokuon = Some(press);
        } else {
            out.after.push((press, map));
        }
    }
    if let Some(p) = t["lengthen"]["token"].as_str().and_then(|x| press_for(l, x)) {
        let map = t["lengthen"]["map"].as_object().into_iter().flatten().map(|(a, b)| (first_char(a), first_char(b.as_str().unwrap_or("")))).collect();
        out.lengthen = Some((p, map));
    }
    let list = |v: &Value| -> Vec<String> { v.as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect() };
    for (a, v) in t["vowel_readings"].as_object().into_iter().flatten() {
        out.vowel_readings.insert(first_char(a), list(v).iter().map(|x| first_char(x)).collect());
    }
    for (a, v) in t["final_readings"].as_object().into_iter().flatten() {
        out.final_readings.insert(first_char(a), list(v));
    }
    out
}

#[derive(Clone, Debug)]
struct TPlan {
    keys: Vec<Press>,
    /// What the preedit shows (no reading from the menu needed).
    default: bool,
    /// Ends in っ read as a final ㅅ: the next syllable's initial must be one っ raises.
    sokuon: bool,
}

impl TPlan {
    fn cost(&self) -> usize {
        self.keys.len() + if self.default { 0 } else { 3 }
    }
}

/// Ways to type an initial and vowel: a kana key's syllable, its initial raised by っ before it or
/// changed by ゛ ゜ after it, its vowel changed by a small vowel or ー, or read otherwise.
/// `after_sokuon`: a っ before it is read as the previous syllable's ㅅ (so the initial stays plain).
fn heads(l: &Layout, tr: &Trans, ini: char, vowel: char, after_sokuon: bool) -> Vec<TPlan> {
    let mut bases: Vec<(Press, char, char)> = vec![];
    for (p, t) in &l.toks {
        let c = first_char(&t.t);
        if t.t.chars().count() != 1 {
            continue;
        }
        if t.r == "x" && ('가'..='힣').contains(&c) && jamo(c).2.is_none() {
            bases.push((*p, jamo(c).0, jamo(c).1));
        } else if t.r == "v" && !p.1 {
            bases.push((*p, 'ㅇ', c)); // い: a vowel on ㅇ
        }
    }
    let smalls: Vec<(Press, char)> = l.toks.iter().filter(|(p, t)| t.r == "v" && p.1).map(|(p, t)| (*p, first_char(&t.t))).collect();
    let mut out = vec![];
    for &(press, l0, v0) in &bases {
        let pres: &[bool] = if after_sokuon || tr.sokuon.is_none() { &[false] } else { &[false, true] };
        for &pre in pres {
            let mut afters: Vec<Option<&(Press, BTreeMap<char, char>)>> = vec![None];
            afters.extend(tr.after.iter().map(Some));
            for post in afters {
                let mut cur = if pre { *tr.raise.get(&l0).unwrap_or(&l0) } else { l0 };
                let mut keys = vec![];
                if pre {
                    keys.push(tr.sokuon.unwrap());
                }
                keys.push(press);
                if let Some((p, map)) = post {
                    let Some(&b) = map.get(&cur) else { continue };
                    cur = if pre { *tr.raise.get(&b).unwrap_or(&b) } else { b };
                    keys.push(*p);
                }
                if cur != ini {
                    continue;
                }
                if after_sokuon && !tr.raise.contains_key(&cur) {
                    continue;
                }
                // The vowel: as is, or a small vowel (joining where Korean has the compound), then ー.
                let mut vs: Vec<(Vec<Press>, char)> = vec![(vec![], v0)];
                for &(sp, sv) in &smalls {
                    let joined = VOWEL_JOINS.iter().find(|j| j.1 == v0 && j.2 == sv).map_or(sv, |j| j.0);
                    vs.push((vec![sp], joined));
                }
                if let Some((lp, map)) = &tr.lengthen {
                    for (k, v) in vs.clone() {
                        if let Some(&long) = map.get(&v) {
                            vs.push(([k, vec![*lp]].concat(), long));
                        }
                    }
                }
                for (vk, v) in vs {
                    let read = tr.vowel_readings.get(&v).is_some_and(|r| r.contains(&vowel));
                    if v != vowel && !read {
                        continue;
                    }
                    out.push(TPlan { keys: [keys.clone(), vk].concat(), default: v == vowel && !after_sokuon, sokuon: false });
                }
            }
        }
    }
    out.sort_by_key(TPlan::cost);
    out
}

/// Ways to type one syllable, cheapest first.
fn transcribed_plans(l: &Layout, tr: &Trans, s: char, after_sokuon: bool) -> Vec<TPlan> {
    let (i, v, f) = jamo(s);
    let hs = heads(l, tr, i, v, after_sokuon);
    let Some(f) = f else { return hs.into_iter().take(6).collect() };
    let mut tails: Vec<TPlan> = l
        .toks
        .iter()
        .filter(|(_, t)| t.t == f.to_string() && (t.r == "f" || t.r == "c"))
        .map(|(p, _)| TPlan { keys: vec![*p], default: true, sokuon: false })
        .collect();
    for r in tr.final_readings.get(&f).into_iter().flatten() {
        if let Some(&(p, _)) = l.toks.iter().find(|(_, t)| t.t == *r && t.r == "f") {
            tails.push(TPlan { keys: vec![p], default: false, sokuon: false });
        } else if Some(r.as_str()) == l.json["transcription"]["modifiers"].as_object().and_then(|m| m.keys().find(|k| m[*k]["position"] == "before")).map(|k| k.as_str()) {
            tails.push(TPlan { keys: vec![tr.sokuon.unwrap()], default: false, sokuon: true });
        } else {
            let c = first_char(r);
            if let Some(h) = heads(l, tr, jamo(c).0, jamo(c).1, false).into_iter().find(|h| h.default) {
                tails.push(TPlan { keys: h.keys, default: false, sokuon: false });
            }
        }
    }
    let mut out = vec![];
    for h in hs.iter().take(6) {
        for t in &tails {
            out.push(TPlan { keys: [h.keys.clone(), t.keys.clone()].concat(), default: h.default && t.default, sokuon: t.sokuon });
        }
    }
    out.sort_by_key(TPlan::cost);
    out.truncate(6);
    out
}

/// Types a Korean word on layout 19: plans from the JSON, cheapest first; plans that need no
/// reading are checked syllable by syllable against the preedit, the whole word against the menu.
fn type_transcribed(d: &Arc<Data>, l: &Layout, tr: &Trans, word: &str) -> Result<usize, Fail> {
    let syl: Vec<char> = word.chars().collect();
    for (k, &s) in syl.iter().enumerate() {
        if transcribed_plans(l, tr, s, false).is_empty() && transcribed_plans(l, tr, s, true).is_empty() {
            return Err(Fail::Untypeable(k, "no keys for it".into()));
        }
    }
    struct Search<'a> {
        d: &'a Arc<Data>,
        l: &'a Layout,
        tr: &'a Trans,
        syl: Vec<char>,
        checks: usize,
        first: Option<String>,
    }
    fn dfs(s: &mut Search, k: usize, keys: &[Press], default: bool, sokuon: bool) -> Option<usize> {
        if s.checks > 80 {
            return None;
        }
        if k == s.syl.len() {
            s.checks += 1;
            let word: String = s.syl.iter().collect();
            let (shown, menu) = screen(s.d, s.l.id, keys);
            if s.first.is_none() {
                s.first = Some(shown.clone());
            }
            if shown == word {
                return Some(0);
            }
            return menu.iter().take(TOP).position(|m| *m == word);
        }
        for plan in transcribed_plans(s.l, s.tr, s.syl[k], sokuon) {
            if plan.sokuon && k + 1 == s.syl.len() {
                continue;
            }
            let next = [keys, &plan.keys].concat();
            let def = default && plan.default && !plan.sokuon;
            if def {
                s.checks += 1;
                let want: String = s.syl[..=k].iter().collect();
                if screen(s.d, s.l.id, &next).0 != want {
                    continue;
                }
            }
            if let Some(row) = dfs(s, k + 1, &next, def, plan.sokuon) {
                return Some(row);
            }
        }
        None
    }
    let mut search = Search { d, l, tr, syl, checks: 0, first: None };
    match dfs(&mut search, 0, &[], true, false) {
        Some(row) => Ok(row),
        None => Err(Fail::NotOffered(search.first.unwrap_or_default())),
    }
}

// ---------- spelled: English, Korean romanization, kana, romaji ----------

/// The press typing a character, from the layout's tokens.
fn press_for(l: &Layout, c: &str) -> Option<Press> {
    l.toks.iter().find(|(_, t)| t.t == c).map(|(p, _)| *p)
}

fn spell(l: &Layout, letters: &str) -> Option<Vec<Press>> {
    letters.chars().map(|c| press_for(l, &c.to_string())).collect()
}

/// Layout 18: the word in its spelling table, syllables run together, else split by `break`.
fn roman_plans(l: &Layout, word: &str) -> Option<Vec<Vec<Press>>> {
    let t = &l.json["spelling"];
    let get = |table: &str, j: char| t[table][j.to_string()].as_str().map(String::from);
    let mut syls = vec![];
    for s in word.chars() {
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

/// Layout 20: each kana on its key, a voiced one as its plain kana and ゛ (゜).
fn kana_plan(l: &Layout, reading: &str) -> Option<Vec<Press>> {
    let mut out = vec![];
    for c in reading.chars() {
        if let Some(p) = l.presses(&c.to_string(), &["k"]).first() {
            out.push(*p);
            continue;
        }
        let n = c as u32;
        let (base, mark) = if c == 'ゔ' {
            ('う', "゛")
        } else if let Some(b) = char::from_u32(n - 1).filter(|b| l.presses(&b.to_string(), &["k"]).len() > 0 && "がぎぐげござじずぜぞだぢづでどばびぶべぼ".contains(c)) {
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

/// Layout 21: the reading in its spelling table, two kana at once where the table has them; っ
/// doubles the next consonant.
fn romaji_plan(l: &Layout, reading: &str) -> Option<Vec<Press>> {
    let t = &l.json["spelling"];
    let kana: Vec<char> = reading.chars().collect();
    let mut letters = String::new();
    let mut i = 0;
    let mut sokuon = false;
    while i < kana.len() {
        if kana[i] == 'っ' {
            if sokuon {
                letters += t["っ"].as_str()?;
            }
            sokuon = true;
            i += 1;
            continue;
        }
        let two: String = kana[i..(i + 2).min(kana.len())].iter().collect();
        let (mut r, n) = match t[&two].as_str() {
            Some(r) if two.chars().count() == 2 => (r.to_string(), 2),
            _ => (t[kana[i].to_string()].as_str()?.to_string(), 1),
        };
        // ん before a vowel, y or n: `nn` would start the next kana on its second n (ぜんにん).
        if kana[i] == 'ん' {
            let next = kana.get(i + 1).and_then(|k| t[k.to_string()].as_str()).and_then(|s| s.chars().next());
            if next.is_some_and(|c| "aiueoyn".contains(c)) {
                r = t["ん'"].as_str()?.to_string();
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

/// A converting layout: the wanted text must show, or be in the first `TOP` rows.
fn offered(d: &Arc<Data>, l: &Layout, plans: &[Vec<Press>], want: &str) -> Result<usize, Fail> {
    let mut first = String::new();
    for plan in plans {
        let (shown, menu) = screen(d, l.id, plan);
        if shown == want {
            return Ok(0);
        }
        if let Some(row) = menu.iter().take(TOP).position(|m| m == want) {
            return Ok(row);
        }
        if first.is_empty() {
            first = shown;
        }
    }
    Err(Fail::NotOffered(first))
}

fn type_item(d: &Arc<Data>, l: &Layout, item: &Item) -> Result<usize, Fail> {
    let untypeable = || Fail::Untypeable(0, String::new());
    match l.input.as_str() {
        // Muhenkan: the keys go to the app as they are, so every character needs a key.
        "direct" => spell(l, &item.text).map(|_| 0).ok_or_else(untypeable),
        "compose" => type_composed(d, l, &item.text),
        "roman" => offered(d, l, &roman_plans(l, &item.text).ok_or_else(untypeable)?, &item.text),
        "kana" => offered(d, l, &[kana_plan(l, &item.reading).ok_or_else(untypeable)?], &item.text),
        "romaji" => offered(d, l, &[romaji_plan(l, &item.reading).ok_or_else(untypeable)?], &item.text),
        "transcription" => type_transcribed(d, l, &trans(l), &item.text),
        _ => Err(untypeable()),
    }
}

// ---------- the test ----------

#[derive(Default)]
struct Tally {
    items: usize,
    top1: usize,
    found: usize,
    untypeable: Vec<String>,
    commit: Vec<String>,
    not_offered: Vec<String>,
    /// Syllables no plan could type, with how often.
    stuck: BTreeMap<char, usize>,
}

/// Release builds only (`cargo test --release`): in debug it takes minutes.
#[test]
#[cfg_attr(debug_assertions, ignore)]
fn robot_types_the_corpora() {
    let d = data();
    let mut report = String::from("# Typing robot report\n\nGenerated by `engine/tests/robot.rs` (docs/robot.md).\n\n");
    let mut failures = vec![];
    // Claims weaker than what the layout earns: allowed, but warned about.
    let mut warnings: Vec<(String, String)> = vec![];
    let mut table = String::from("| Layout | Input | Corpus | Items | Typed / found | Row 0 | Untypeable | Commit mid-word | Not in top 5 |\n|---|---|---|---|---|---|---|---|---|\n");
    let mut claims = String::from("| Layout | Language | Claims | Earns | Result |\n|---|---|---|---|---|\n");
    let mut examples = String::new();
    let mut skipped = BTreeMap::new();
    for l in layouts() {
        if l.languages.is_empty() {
            continue;
        }
        for lang in l.reliability.keys().filter(|k| !l.languages.contains(k)) {
            failures.push(format!("{} claims a reliability for {lang}, which it doesn't declare", l.file));
        }
        for lang in &l.languages {
            let claim = l.reliability.get(lang).map(String::as_str).unwrap_or("");
            if !LEVELS.contains(&claim) {
                failures.push(format!("{} declares {lang} with reliability {claim:?}: one of {LEVELS:?}", l.file));
            }
            // The weakest result on the common words and on the other corpora.
            let (mut common_rate, mut corpus_rate) = (1.0f64, 1.0f64);
            for c in corpora(lang) {
                let mut t = Tally::default();
                for item in &c.items {
                    t.items += 1;
                    let label = if item.reading != item.text { format!("{} ({})", item.text, item.reading) } else { item.text.clone() };
                    match type_item(&d, &l, item) {
                        Ok(row) => {
                            t.found += 1;
                            t.top1 += (row == 0) as usize;
                        }
                        Err(Fail::Untypeable(k, shown)) => {
                            if let Some(c) = item.text.chars().nth(k) {
                                *t.stuck.entry(c).or_default() += 1;
                            }
                            t.untypeable.push(format!("{label}: syllable {} shows {shown:?}", k + 1));
                        }
                        Err(Fail::CommitMidWord(k)) => t.commit.push(format!("{label}: commit before syllable {}", k + 1)),
                        Err(Fail::NotOffered(first)) => t.not_offered.push(format!("{label}: row 0 {first:?}")),
                    }
                }
                let rate = t.found as f64 / t.items.max(1) as f64;
                let worst = if c.common { &mut common_rate } else { &mut corpus_rate };
                *worst = worst.min(rate);
                writeln!(
                    table,
                    "| {} | {} | {} | {} | {:.1}% | {:.1}% | {} | {} | {} |",
                    l.file, l.input, c.name, t.items, rate * 100.0, t.top1 as f64 / t.items.max(1) as f64 * 100.0,
                    t.untypeable.len(), t.commit.len(), t.not_offered.len(),
                ).unwrap();
                skipped.insert(c.name.clone(), (c.items.len(), c.skipped));
                if !t.stuck.is_empty() {
                    let mut stuck: Vec<_> = t.stuck.iter().collect();
                    stuck.sort_by(|a, b| b.1.cmp(a.1));
                    let top: Vec<String> = stuck.iter().take(30).map(|(c, n)| format!("{c} {n}")).collect();
                    writeln!(examples, "\n### {} · {} · syllables it can't type\n\n{}", l.file, c.name, top.join(" · ")).unwrap();
                }
                for (kind, list) in [("Untypeable", &t.untypeable), ("Commit mid-word", &t.commit), ("Not in the first 5 rows", &t.not_offered)] {
                    if !list.is_empty() {
                        writeln!(examples, "\n### {} · {} · {kind} ({})\n", l.file, c.name, list.len()).unwrap();
                        for e in list.iter().take(25) {
                            writeln!(examples, "- {e}").unwrap();
                        }
                    }
                }
            }
            let earns = if common_rate == 1.0 && corpus_rate == 1.0 {
                "perfect"
            } else if common_rate == 1.0 && corpus_rate >= PRODUCTION_RATE {
                "production"
            } else {
                "experimental"
            };
            let rank = |x: &str| LEVELS.iter().position(|&y| y == x);
            let pass = rank(earns) >= rank(claim) && rank(claim).is_some();
            let under = pass && rank(earns) > rank(claim);
            let verdict = if !pass { "**fail**" } else if under { "pass (claims less: warning)" } else { "pass" };
            writeln!(claims, "| {} | {lang} | {claim} | {earns} | {verdict} |", l.file).unwrap();
            if under {
                warnings.push((l.file.clone(), format!("{} claims {claim} for {lang} but earns {earns}: the claim could be raised", l.file)));
            }
            if !pass && rank(claim).is_some() {
                failures.push(format!(
                    "{} claims {claim} for {lang} but earns {earns} (common words {:.1}%, other corpora at worst {:.1}%)",
                    l.file, common_rate * 100.0, corpus_rate * 100.0
                ));
            }
        }
    }
    report += "## Claims\n\n`perfect`: every item of every corpus. `production`: every common word, and 80% of each other corpus. `experimental`: no bar.\n\n";
    report += &claims;
    for (_, w) in &warnings {
        writeln!(report, "\n> **Warning:** {w}").unwrap();
    }
    report += "\n## Corpora by layout\n\n";
    report += &table;
    report += "\n## Corpora\n\nDistinct items typed, and characters left out (no word of the language: punctuation, digits, Latin in Korean …).\n\n";
    for (name, (n, s)) in &skipped {
        writeln!(report, "- {name}: {n} items, {s} characters left out").unwrap();
    }
    report += "\n## Failing examples\n";
    report += &examples;
    let path = root("target/robot-report.md");
    std::fs::write(&path, &report).unwrap();
    eprintln!("{table}\n{claims}\nreport: {path}");
    for (file, w) in &warnings {
        eprintln!("warning: {w}");
        if std::env::var_os("GITHUB_ACTIONS").is_some() {
            // An annotation on the run and the file (GitHub workflow command).
            println!("::warning file=docs/layouts/{file},title=Reliability claim too conservative::{w}");
        }
    }
    assert!(failures.is_empty(), "robot failures:\n{}", failures.join("\n"));
}

// ---------- the JSON is the engine's ----------

/// Every key a layout file lists types what the file says in the engine (docs/robot.md: the robot
/// trusts the files, so they must not drift from the code).
#[test]
fn json_matches_engine() {
    use bada::layout::{self, Role};
    let mut wrong = vec![];
    for l in layouts() {
        for ((key, shift), tok) in &l.toks {
            let c = tok.t.chars().next().unwrap_or(' ');
            let ok = match l.input.as_str() {
                "compose" => match layout::role(l.id, *key, *shift) {
                    Some(Role::Cons { ini, fin }) => match tok.r.as_str() {
                        "c" => ini == Some(c) && fin == Some(c),
                        "i" => ini == Some(c),
                        "f" => fin == Some(c),
                        _ => false,
                    },
                    Some(Role::ConsA(x)) => tok.r == "a" && x == c,
                    Some(Role::Vowel(x)) => tok.r == "v" && x == c,
                    Some(Role::VowelNew(x)) => tok.r == "n" && x == c,
                    Some(Role::Lit(x)) => tok.r == "x" && x == c,
                    None => false,
                },
                // 15's own keys (q ch, x ㅇ, `;` the break, ¥ ₩ …): as the engine reads them.
                "roman" if l.id == 15 => bada::bada_roman::token(*key, *shift).is_some_and(|(t, r)| t == tok.t && r == tok.r),
                "direct" | "roman" | "romaji" | "search" => layout::jis_char(*key, *shift) == Some(c),
                "kana" => bada::japanese::jis_kana(*key, *shift) == Some(c),
                _ => true,
            };
            if !ok {
                wrong.push(format!("{} {}{}: {:?}", l.file, if *shift { "⇧" } else { "" }, key.code(), tok));
            }
        }
    }
    // 19's transcription map: ゛ ゜ after a kana and っ before it change the initial as it says.
    for l in layouts().iter().filter(|l| l.input == "transcription") {
        let tr = trans(l);
        let syl = |presses: &[Press]| {
            let p: Vec<layout::Press> = presses.iter().map(|&(k, s)| layout::Press::new(k, s)).collect();
            bada::kana_korean::compose_presses(&p).chars().next().and_then(|c| ('가'..='힣').contains(&c).then(|| jamo(c).0))
        };
        for (press, tok) in &l.toks {
            let c = first_char(&tok.t);
            if tok.r != "x" || !('가'..='힣').contains(&c) || press.1 {
                continue;
            }
            let ini = jamo(c).0;
            for (m, map) in &tr.after {
                if let Some(&want) = map.get(&ini) {
                    if syl(&[*press, *m]) != Some(want) {
                        wrong.push(format!("{} {} then {}: not {want}", l.file, tok.t, m.0.code()));
                    }
                }
            }
            if let (Some(s), Some(&want)) = (tr.sokuon, tr.raise.get(&ini)) {
                if syl(&[s, *press]) != Some(want) {
                    wrong.push(format!("{} っ then {}: not {want}", l.file, tok.t));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "layout files that disagree with the engine:\n{}", wrong.join("\n"));
}




// ---------- the pages say what the JSON claims ----------

/// Each layout page's metadata header states the reliability its JSON claims, per language
/// (docs/layouts/README.md), and so does the README's table.
#[test]
fn pages_state_the_claims() {
    let name = |l: &str| match l {
        "english" => "English",
        "korean" => "Korean",
        "japanese_kana" => "Japanese (kana)",
        "japanese_romaji" => "Japanese (romaji)",
        _ => "?",
    };
    let readme = std::fs::read_to_string(root("../docs/layouts/README.md")).unwrap();
    let mut wrong = vec![];
    for l in layouts() {
        let page = l.file.replace(".json", ".md");
        let text = std::fs::read_to_string(root(&format!("../docs/layouts/{page}"))).unwrap();
        let Some(row) = text.lines().find(|r| r.starts_with("| **Reliability** |")) else {
            wrong.push(format!("{page}: no Reliability row"));
            continue;
        };
        let claims: Vec<(String, String)> = l.languages.iter().map(|g| (name(g).to_string(), l.reliability.get(g).cloned().unwrap_or_default())).collect();
        for (lang, level) in &claims {
            if !row.contains(&format!("{lang}: **{level}**")) {
                wrong.push(format!("{page}: Reliability should say {lang}: **{level}**"));
            }
        }
        if claims.is_empty() && !row.contains("none") {
            wrong.push(format!("{page}: Reliability should say none"));
        }
        let line = readme.lines().find(|r| r.starts_with(&format!("| {} | [", l.id))).unwrap_or("");
        let want = if claims.is_empty() { "—".to_string() } else { claims.iter().map(|(g, v)| format!("{g}: {v}")).collect::<Vec<_>>().join(" · ") };
        if !line.ends_with(&format!(" {want} |")) {
            wrong.push(format!("README.md row {}: Reliability should be {want}", l.id));
        }
    }
    assert!(wrong.is_empty(), "layout pages that disagree with their JSON:\n{}", wrong.join("\n"));
}
