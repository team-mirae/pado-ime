//! The engine: muhenkan and henkan modes (spec §4), the signed suggestion menu (§6.4), the keys
//! (§6), F-key actions (§6.5) and Bada's actions (docs/layouts/13-bada.md §6).
//!
//! Event in → output out. The shell owns input-source switching (英数, かな, HYPER/MEH+かな) and
//! the host text; the engine says what to commit, what to show and whether it took the key.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::data::Data;
use crate::japanese;
use crate::key::{Key, Mods};
use crate::keymodel::KeyModels;
use crate::layout::{self, BADA, BADA_PLAIN, BADA_ROMAN, K3S_JIS, KANA, KANA_KOREAN, KOREAN_ROMAN, KTS, LEGACY_KOREAN, LEGACY_MOONSHOT, Press, QWERTY, ROMAJI, SYMBOLS, UNICODE};
use crate::userdict::{Language, Strength, UserDict};
use crate::width::{Width, convert};

/// The user's settings (spec §5.3, §6.4, §6.5), as the shell stores them.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    /// Henkan candidates, in the user's order (earlier wins ties): profile `mac-kana`'s own list.
    pub enabled: Vec<u32>,
    /// Henkan profiles (§6.6): `mac-kana` (かな), `henkan` (変換), `katakana` (カタカナ). A profile
    /// missing here: `mac-kana` uses `enabled`, the others link to `mac-kana`.
    pub profiles: BTreeMap<String, Profile>,
    /// Layout id → weight (§5.3): every entry that layout raises has its score multiplied by it.
    /// Missing: 1. Clamped to 0–10.
    pub weights: HashMap<u32, f64>,
    pub default_muhenkan: u32,
    /// Negative menu entries (§6.4: −1 hiragana, −2 katakana for 20 and 21). Off by default.
    pub script_forms: bool,
    /// Emoji skin tones in the order layout 150 offers them ("none", "light", "medium-light",
    /// "medium", "medium-dark", "dark"); a tone left out isn't offered.
    pub skin_tones: Vec<String>,
    /// The one method that contributes negative menu entries (§6.4). None: the lowest enabled id
    /// that offers them.
    pub negative_method: Option<u32>,
    /// Intercept F6–F10 at all (§6.5).
    pub intercept_f: bool,
    /// "F6" … "F10" → "swallow" | "pass" | "<method>.<action>". Missing: swallow.
    pub fkeys: HashMap<String, String>,
    /// Layout 15: "Accept Japanese romaji readings as shortcuts when it saves keystrokes" (mei 명).
    pub ja_shortcuts: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            enabled: vec![BADA_PLAIN],
            profiles: BTreeMap::new(),
            weights: HashMap::new(),
            default_muhenkan: QWERTY,
            script_forms: false,
            skin_tones: crate::search::TONES.iter().map(|t| t.to_string()).collect(),
            negative_method: None,
            intercept_f: true,
            fkeys: HashMap::new(),
            ja_shortcuts: true,
        }
    }
}

/// Results a search layout (150, 151) lists, at most.
const SEARCH_RESULTS: usize = 40;

/// Typo suggestions listed under layout 13's reading, at most.
const TYPO_SUGGESTIONS: usize = 3;
/// Japanese: rows given to a reading's most common words (`ja-common-words.txt`).
const COMMON_ROWS: usize = 3;
/// Japanese: conversions of a reading the menu lists, at most.
const CONVERSIONS: usize = 6;
/// Japanese: how far down the conversions the user's own words are looked for.
const USER_CONVERSIONS: usize = 32;

/// The key models (§7.0, keymodel.rs). A layout's evidence is KEY_POINTS for every nat of its share
/// among the layouts that read the keys: one ten times less likely than the rest loses about 11.
const KEY_POINTS: f64 = 5.0;
/// What a word that goes on after the last staged key counts for, beside one that ends there. A
/// commit only ever sees a word that has ended; while it's being typed, the keys so far say more.
const KEY_GOES_ON: f64 = 0.02;
/// The most a layout loses to the key models: less than SET_SCORE, so its first reading stays
/// above zero, where a layout weighted 0 is.
const KEY_FLOOR: f64 = 60.0;
/// In a set, where every layout's first reading starts before its evidence; more when the tiers
/// know it as a word (85 and up), more again when the word is the user's own (§8).
const SET_SCORE: f64 = 80.0;
const SET_KNOWN: f64 = 5.0;
const SET_OWN: f64 = 40.0;
/// A word and its particle (§7.0): how likely a staged run is two layouts' keys, an English word
/// and then a Korean or Japanese particle, beside one layout's all through: after a known English
/// word, and after a capitalised name the word lists don't have.
const SPLIT_KNOWN: f64 = 0.5;
const SPLIT_NAME: f64 = 0.05;

/// A henkan profile (§6.6): its own candidates, or a link to another profile.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct Profile {
    /// Another profile whose candidates this one uses ("mac-kana", "henkan", "katakana").
    pub link: Option<String>,
    /// This profile's own candidates, in order (unused while it links).
    pub enabled: Vec<u32>,
    /// This profile's own F keys (§6.5); None: `mac-kana`'s (the settings' `intercept_f`, `fkeys`).
    pub f_keys: Option<FKeys>,
}

/// F6–F10 for one profile (§6.5): intercepted or not, and each key's binding.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct FKeys {
    pub intercept: bool,
    /// "F6" … "F10" → "swallow" | "pass" | "<method>.<action>". Missing: swallow.
    pub keys: HashMap<String, String>,
}

/// The henkan profiles (§6.6), in the order the settings show them.
pub const PROFILES: [&str; 3] = ["mac-kana", "henkan", "katakana"];

/// The F-key actions each method publishes (§6.5): (method id, action id, label).
pub const ACTIONS: &[(u32, &str, &str)] = &[
    (
        BADA,
        "raw",
        "Raw jamo: every key as its jamo, nothing composed",
    ),
    (BADA, "prediction-1", "Use the 1st prediction"),
    (BADA, "prediction-2", "Use the 2nd prediction"),
    (BADA, "prediction-3", "Use the 3rd prediction"),
    (BADA, "prediction-4", "Use the 4th prediction"),
    (BADA, "prediction-5", "Use the 5th prediction"),
    (BADA, "qwerty-fullwidth", "QWERTY, full-width"),
    (BADA, "qwerty-halfwidth", "QWERTY, half-width"),
    (
        BADA,
        "hanja",
        "Hanja: staged text, else selection, else OECD countries",
    ),
    (BADA_ROMAN, "initials", "Initial consonants only: staged text, else selection (한국 → ㅎㄱ)"),
    (BADA_ROMAN, "vowels", "Vowels only: staged text, else selection (한국 → ㅏㅜ)"),
    (BADA_ROMAN, "finals", "Final consonants only, syllables without one dropped: staged text, else selection (한국 → ㄴㄱ)"),
    (BADA_ROMAN, "fullwidth-latin", "Full-width Latin (the keys as QWERTY)"),
    (BADA_ROMAN, "halfwidth-latin", "Half-width Latin (the keys as QWERTY)"),
    (BADA_PLAIN, "qwerty-fullwidth", "QWERTY, full-width"),
    (BADA_PLAIN, "qwerty-halfwidth", "QWERTY, half-width"),
    (BADA_PLAIN, "initials", "Initial consonants only: staged text, else selection (한국 → ㅎㄱ)"),
    (BADA_PLAIN, "vowels", "Vowels only: staged text, else selection (한국 → ㅏㅜ)"),
    (BADA_PLAIN, "finals", "Final consonants only, syllables without one dropped: staged text, else selection (한국 → ㄴㄱ)"),
    (BADA_PLAIN, "commit-bare", "Commit as typed, implied ㅇ dropped (가어 → 가ㅓ)"),
    (LEGACY_KOREAN, "commit", "Commit as typed (this layout's reading, no ranking)"),
    (LEGACY_KOREAN, "commit-bare", "Commit as typed, implied ㅇ dropped (가어 → 가ㅓ)"),
    (LEGACY_MOONSHOT, "commit", "Commit as typed (this layout's reading, no ranking)"),
    (LEGACY_MOONSHOT, "commit-bare", "Commit as typed, implied ㅇ dropped (가어 → 가ㅓ)"),
    (KANA, "hiragana", "Hiragana"),
    (KANA, "katakana", "Katakana"),
    (KANA, "halfwidth-katakana", "Half-width katakana"),
    (
        KANA,
        "fullwidth-latin",
        "Full-width Latin (the keys as QWERTY)",
    ),
    (
        KANA,
        "halfwidth-latin",
        "Half-width Latin (the keys as QWERTY)",
    ),
    (ROMAJI, "hiragana", "Hiragana"),
    (ROMAJI, "katakana", "Katakana"),
    (ROMAJI, "halfwidth-katakana", "Half-width katakana"),
    (
        ROMAJI,
        "fullwidth-latin",
        "Full-width Latin (the letters typed)",
    ),
    (
        ROMAJI,
        "halfwidth-latin",
        "Half-width Latin (the letters typed)",
    ),
];

/// Layout 15: what a reading that departs from the spelling costs, per syllable it changes (ln of how
/// many times better known it must be: e^3 ≈ 20), and at most.
const BADA_ROMAN_VARIANT_COST: f64 = 3.0;
const BADA_ROMAN_VARIANT_MAX: f64 = 10.0;
/// Layout 15, no reading known: the jamo model's cost per changed syllable (log10), and how much more
/// likely (log10) a reading must look than the spelling's to show first.
const BADA_ROMAN_LM_VARIANT: f64 = 2.5;
const BADA_ROMAN_LM_MARGIN: f64 = 2.0;

/// How far two readings of the same letters are apart: edit distance over their jamo, a silent
/// initial ㅇ left out. Splitting the letters elsewhere costs nothing (기어비 / 기업이), reading a
/// letter as another jamo does (종 / 정: 1).
fn syllable_distance(a: &str, b: &str) -> usize {
    let jamo = |s: &str| -> Vec<char> {
        let syls: Vec<Result<(char, char, Option<char>), char>> =
            s.chars().map(|c| crate::hangul::decompose(c).ok_or(c)).collect();
        let mut out = Vec::new();
        for (n, syl) in syls.iter().enumerate() {
            match *syl {
                Ok((l, v, f)) => {
                    // ㅀ + ㄷ is how `lt` reads with its implied ㅎ (bada_roman.rs): the same
                    // letters as ㄹ + ㅌ
                    let implied = |at: usize| matches!(syls.get(at), Some(Ok((_, _, Some('ㅀ')))));
                    let next_d = matches!(syls.get(n + 1), Some(Ok(('ㄷ', _, _))));
                    let l = if l == 'ㄷ' && n > 0 && implied(n - 1) { 'ㅌ' } else { l };
                    if l != 'ㅇ' {
                        out.push(l);
                    }
                    out.push(v);
                    out.extend(if f == Some('ㅀ') && next_d { Some('ㄹ') } else { f });
                }
                Err(c) => out.push(c),
            }
        }
        out
    };
    let (a, b) = (jamo(a), jamo(b));
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut prev = row[0];
        row[0] = i;
        for j in 1..=b.len() {
            let cur = row[j];
            row[j] = (row[j] + 1).min(row[j - 1] + 1).min(prev + usize::from(a[i - 1] != b[j - 1]));
            prev = cur;
        }
    }
    row[b.len()]
}

/// The actions of layouts 15 and 16 that keep one jamo of each syllable.
fn keep(action: &str) -> Option<crate::bada_roman::Keep> {
    use crate::bada_roman::Keep;
    match action {
        "initials" => Some(Keep::Initials),
        "vowels" => Some(Keep::Vowels),
        "finals" => Some(Keep::Finals),
        _ => None,
    }
}

/// Methods that offer script forms as negative menu entries (§6.4).
const SCRIPT_FORM_METHODS: &[u32] = &[KANA, ROMAJI];

/// A Japanese method's kana reading of the presses.
fn kana_reading(method: u32, presses: &[Press]) -> String {
    if method == KANA {
        japanese::kana_compose(presses)
    } else {
        japanese::romaji_compose(presses)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Muhenkan,
    Henkan,
}

/// What the engine asks the shell to do with one event.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Output {
    /// Text to insert (replacing the marked text, or the selection if `replace_selection`).
    pub commit: String,
    pub replace_selection: bool,
    /// The marked text after the commit.
    pub preedit: String,
    /// The menu's rows, top to bottom (negative entries first); empty: no menu.
    pub menu: Vec<String>,
    /// The highlighted row, and the row of entry 0.
    pub selected: usize,
    pub zero: usize,
    /// The engine took the key; false: the app gets it (after the commit).
    pub consumed: bool,
    /// The menu is one row of single letters (⌥ + letter variants); `selected` past its end: none.
    pub horizontal: bool,
    /// Per menu row: a note shown in gray at the right (romaji, a kana reading), and whether the
    /// row is Japanese (a Japanese font). Empty when no row has one.
    pub notes: Vec<String>,
    pub japanese: Vec<bool>,
    /// The engine needs the host's selection to finish: "hanja", "full" or "half". The shell calls
    /// `host_selection` with it.
    pub need_selection: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Override {
    Raw,
    Qwerty(Width),
    /// A Japanese method's reading in a script: hiragana, katakana, or half-width katakana.
    Kana {
        method: u32,
        script: KanaScript,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KanaScript {
    Hiragana,
    Katakana,
    HalfKatakana,
}

fn in_script(hiragana: &str, script: KanaScript) -> String {
    match script {
        KanaScript::Hiragana => hiragana.to_string(),
        KanaScript::Katakana => japanese::katakana(hiragana),
        KanaScript::HalfKatakana => convert(&japanese::katakana(hiragana), Width::Half),
    }
}

#[derive(Clone, Debug, Default)]
struct Entry {
    text: String,
    score: f64,
    /// Shown in gray at the row's right: romaji for kana, the kana reading for kanji.
    note: String,
    /// A Japanese reading: drawn in a Japanese font.
    japanese: bool,
    /// Where it came from: typo suggestions go under layout 13's reading.
    origin: Origin,
    /// The layout that raised it (None: added after the layouts, as typo suggestions).
    source: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Origin {
    #[default]
    Other,
    /// Layout `id`'s composition of the presses.
    Composed(u32),
}

impl Entry {
    fn new(text: String, score: f64) -> Entry {
        Entry { text, score, ..Entry::default() }
    }

    /// A kana entry, its romaji as the note.
    fn kana(text: String, score: f64) -> Entry {
        let note = crate::romaji::romaji(&text).unwrap_or_default();
        Entry { text, score, note, japanese: true, origin: Origin::Other, source: None }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PickTarget {
    /// Replace the staged text: commit `before` + the pick.
    Staged { before: String },
    /// Replace the host's selection: `before` + the pick.
    Selection { before: String },
    /// Insert at the caret.
    Insert,
}

#[derive(Clone, Debug)]
struct Pick {
    rows: Vec<(String, String)>,
    index: usize,
    target: PickTarget,
}

#[derive(Clone)]
pub struct Engine {
    data: Arc<Data>,
    settings: Settings,
    mode: Mode,
    /// Text frozen in the preedit (半角/全角 applied to what showed), before the presses.
    prefix: String,
    presses: Vec<Press>,
    over: Option<Override>,
    entries: Vec<Entry>,
    negatives: Vec<Entry>,
    /// Signed index into the menu (§6.4): 0 is the top entry, negatives are script forms.
    index: i32,
    pick: Option<Pick>,
    /// A row picked in the menu (§6.4) and its layout: typing on keeps it, as the row that extends
    /// it, else that layout's best (inter → interpolate when QWERTY's row was picked).
    chosen: Option<(String, Option<u32>)>,
    /// The layout whose reading was fixed last (§6.4): its reading of the keys typed after shows.
    stuck: Option<u32>,
    /// Waiting for the host's selection for this action.
    waiting: Option<String>,
    /// Muhenkan's open ⌥ + letter variants list.
    held: Option<crate::hold::Held>,
    /// A dictionary pinned to this engine (tests); None: the process's (`userdict::current`).
    pinned: Option<Option<std::sync::Arc<UserDict>>>,
    /// The henkan profile in use (§6.6).
    profile: String,
}

impl Engine {
    pub fn new(data: Arc<Data>) -> Engine {
        Engine {
            data,
            settings: Settings::default(),
            mode: Mode::Henkan,
            prefix: String::new(),
            presses: Vec::new(),
            over: None,
            entries: Vec::new(),
            negatives: Vec::new(),
            index: 0,
            pick: None,
            chosen: None,
            stuck: None,
            waiting: None,
            held: None,
            pinned: None,
            profile: PROFILES[0].to_string(),
        }
    }

    pub fn set_settings(&mut self, settings: Settings) {
        self.settings = settings;
        self.settings.enabled.retain(|&id| layout::is_known(id));
        for p in self.settings.profiles.values_mut() {
            p.enabled.retain(|&id| layout::is_known(id));
        }
        self.recompute();
    }

    /// Switches henkan profile (§6.6): what's staged is committed first. Unknown names: `mac-kana`.
    pub fn set_profile(&mut self, name: &str) -> Output {
        let name = if PROFILES.contains(&name) { name } else { PROFILES[0] };
        let mut out = Output::default();
        if name != self.profile {
            out.commit = self.take_shown();
            self.profile = name.to_string();
            self.recompute();
        }
        self.finish(out)
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    /// Whether every candidate in use is a search layout (150, 151): Space then searches.
    fn searching(&self) -> bool {
        let c = self.candidates(&self.profile);
        !c.is_empty() && c.iter().all(|&id| layout::is_search(id))
    }

    /// The candidates of a profile, links followed (a loop or a dead end: `mac-kana`'s own list).
    pub fn candidates(&self, name: &str) -> Vec<u32> {
        let mut at = name.to_string();
        for _ in 0..PROFILES.len() {
            let link = match self.settings.profiles.get(&at) {
                Some(Profile { link: Some(l), .. }) => l.clone(),
                Some(Profile { link: None, enabled, .. }) if at != PROFILES[0] => return enabled.clone(),
                None if at != PROFILES[0] => PROFILES[0].to_string(),
                _ => return self.settings.enabled.clone(),
            };
            if !PROFILES.contains(&link.as_str()) {
                break;
            }
            at = link;
        }
        self.settings.enabled.clone()
    }

    /// Pins a user dictionary to this engine instead of the process's (None in it: no dictionary).
    pub fn pin_dictionary(&mut self, dict: Option<std::sync::Arc<UserDict>>) {
        self.pinned = Some(dict);
        self.recompute();
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Switching mode commits what's staged (the shell inserts the returned text).
    pub fn set_mode(&mut self, mode: Mode) -> Output {
        let mut out = Output::default();
        if mode != self.mode {
            out.commit = match self.held.take() {
                Some(h) => h.shown().to_string(),
                None => self.take_shown(),
            };
        }
        self.mode = mode;
        self.finish(out)
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// What `flush` would commit now (bada-compatible v1.1 `P_state`, §12): the text as it would be
    /// typed, never the presses. The ⌥ + letter variants: the letter or variant that shows; a pick
    /// on staged text: what shows; a pick on the host's selection: nothing (the selection stays);
    /// else what shows, when something is staged. Changes nothing; `flush` uses it.
    pub fn staged(&self) -> String {
        if let Some(h) = &self.held {
            return h.shown().to_string();
        }
        match &self.pick {
            Some(p) => match p.target {
                PickTarget::Staged { .. } => self.shown(),
                _ => String::new(),
            },
            None if self.has_staged() => self.shown(),
            None => String::new(),
        }
    }

    /// Commits whatever shows (focus lost, mode switch, a click): `staged`.
    pub fn flush(&mut self) -> Output {
        let mut out = Output::default();
        out.commit = self.staged();
        if self.held.take().is_some() {
            return self.finish(out);
        }
        match self.pick.take() {
            Some(p) if !matches!(p.target, PickTarget::Staged { .. }) => {}
            _ => self.clear(),
        }
        self.finish(out)
    }

    fn has_staged(&self) -> bool {
        !self.prefix.is_empty() || !self.presses.is_empty()
    }

    /// The text that shows: the prefix and the entry at the index.
    fn shown(&self) -> String {
        let entry = if self.index < 0 {
            self.negatives.get((-self.index - 1) as usize)
        } else {
            self.entries.get(self.index as usize)
        };
        format!("{}{}", self.prefix, entry.map_or("", |e| e.text.as_str()))
    }

    /// Whether the entry that shows is layout `id`'s.
    fn shows(&self, id: u32) -> bool {
        self.index >= 0 && self.entries.get(self.index as usize).is_some_and(|e| e.source == Some(id))
    }

    fn clear(&mut self) {
        self.prefix.clear();
        self.presses.clear();
        self.over = None;
        self.entries.clear();
        self.negatives.clear();
        self.index = 0;
        self.pick = None;
        self.chosen = None;
        self.stuck = None;
    }

    fn take_shown(&mut self) -> String {
        let text = self.shown();
        self.clear();
        text
    }

    // ---------- readings ----------

    /// How often a Korean word is seen: the built-in count plus the user dictionary's.
    fn ko_count(&self, user: &Option<std::sync::Arc<UserDict>>, word: &str) -> u32 {
        self.data.korean_count(word) + user.as_ref().map_or(0, |u| u.count(Language::Korean, word))
    }

    /// The key models' evidence for each candidate (§7.0): KEY_POINTS × ln of its share when the
    /// staged keys are scored under the model of every candidate that reads them. None for a
    /// candidate that doesn't read the keys or has no model (19, the searches), and for all of
    /// them unless two have one: a layout alone is ranked as it always was.
    fn key_evidence(&self, enabled: &[u32]) -> Vec<Option<f64>> {
        let none = vec![None; enabled.len()];
        let Some(models) = KeyModels::built_in() else { return none };
        // A press with ⌥ is the key's shifted JIS character in every layout that reads it (kana:
        // its full-width form), so it says nothing about which layout is meant: the models see
        // the other keys.
        let plain = self.presses.iter().filter(|p| !p.alt);
        let Some(symbols) = plain.map(|&p| crate::keymodel::symbol(p)).collect::<Option<Vec<u8>>>() else {
            return none;
        };
        if symbols.is_empty() {
            return none;
        }
        let scores: Vec<Option<f64>> = enabled
            .iter()
            .map(|&id| {
                let reads = self.presses.iter().all(|&p| layout::takes(id, p));
                // The keys as the start of a word, which ends at the last one or (counted less) goes on.
                let whole = |(keys, end): (f64, f64)| keys + (end.exp() * (1.0 - KEY_GOES_ON) + KEY_GOES_ON).ln();
                models.score(id, &symbols).filter(|_| reads).map(whole)
            })
            .collect();
        if scores.iter().flatten().count() < 2 {
            return none;
        }
        let best = scores.iter().flatten().copied().fold(f64::NEG_INFINITY, f64::max);
        let total = best + scores.iter().flatten().map(|s| (s - best).exp()).sum::<f64>().ln();
        scores.iter().map(|s| s.map(|s| (KEY_POINTS * (s - total)).max(-KEY_FLOOR))).collect()
    }

    /// A word and its particle (§7.0; JIS Moonshot §5.3): the staged keys as an English word on
    /// QWERTY and then a Korean or Japanese particle on another enabled layout (Claude에, report를,
    /// Claudeの). The start is a known English word or a capitalised name (a Japanese particle only
    /// after a name), and the whole isn't an English word itself. Scored by the key models: the
    /// start under QWERTY's and the particle under its layout's, each as a word that ends, against
    /// every layout's reading of all the keys.
    fn word_and_particle(&self, enabled: &[u32]) -> Option<Entry> {
        if !enabled.contains(&QWERTY) || self.over.is_some() || self.presses.iter().any(|p| p.alt) {
            return None;
        }
        let models = KeyModels::built_in()?;
        let symbols = self.presses.iter().map(|&p| crate::keymodel::symbol(p)).collect::<Option<Vec<u8>>>()?;
        let whole = layout::qwerty(&self.presses);
        if self.data.is_english(&whole) || self.data.is_english_rare(&whole) {
            return None;
        }
        let ended = |id: u32, keys: &[u8]| models.score(id, keys).map(|(keys, end)| keys + end);
        // (a known word, the score, the text, Japanese)
        let mut best: Option<(bool, f64, String, bool)> = None;
        for at in 2..self.presses.len() {
            let (head, tail) = self.presses.split_at(at);
            let start = layout::qwerty(head);
            let name = start.starts_with(|c: char| c.is_ascii_uppercase());
            let letters = start.chars().all(|c| c.is_ascii_alphabetic()) || (name && start.chars().all(|c| c.is_ascii_alphanumeric()));
            let known = self.data.is_english(&start);
            if !letters || !(known || name) {
                continue;
            }
            for &id in enabled {
                if !tail.iter().all(|&p| layout::takes(id, p)) {
                    continue;
                }
                let (ending, japanese) = match id {
                    KANA | ROMAJI => (kana_reading(id, tail), true),
                    KTS | LEGACY_KOREAN | LEGACY_MOONSHOT | BADA | K3S_JIS | BADA_PLAIN => (layout::compose_presses(id, tail), false),
                    _ => continue,
                };
                let fits = if japanese { name && crate::particles::japanese(&ending) } else { crate::particles::korean(&ending) };
                let (Some(a), Some(b)) = (ended(QWERTY, &symbols[..at]), ended(id, &symbols[at..])) else { continue };
                let score = a + b + if known { SPLIT_KNOWN } else { SPLIT_NAME }.ln();
                if fits && best.as_ref().is_none_or(|b| (known, score) > (b.0, b.1)) {
                    best = Some((known, score, start.clone() + &ending, japanese));
                }
            }
        }
        let (_, score, text, japanese) = best?;
        // Its share among the readings of all the keys, as `key_evidence` shares them.
        let wholes = enabled.iter().filter(|&&id| self.presses.iter().all(|&p| layout::takes(id, p))).filter_map(|&id| {
            models.score(id, &symbols).map(|(keys, end)| keys + (end.exp() * (1.0 - KEY_GOES_ON) + KEY_GOES_ON).ln())
        });
        let all: Vec<f64> = wholes.chain([score]).collect();
        let top = all.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let total = top + all.iter().map(|s| (s - top).exp()).sum::<f64>().ln();
        Some(Entry { text, score: SET_SCORE + SET_KNOWN + (KEY_POINTS * (score - total)).max(-KEY_FLOOR), japanese, ..Entry::default() })
    }

    fn recompute(&mut self) {
        self.entries.clear();
        self.negatives.clear();
        self.index = 0;
        if self.presses.is_empty() {
            return;
        }
        if let Some(over) = self.over {
            let text = match over {
                Override::Raw => layout::raw(BADA, &self.presses),
                Override::Qwerty(w) => convert(&layout::qwerty(&self.presses), w),
                Override::Kana { method, script } => {
                    in_script(&kana_reading(method, &self.presses), script)
                }
            };
            let entry = match over {
                Override::Kana { .. } => Entry::kana(text, 0.0),
                _ => Entry::new(text, 0.0),
            };
            self.entries.push(entry);
            return;
        }
        let enabled = self.candidates(&self.profile);
        let evidence = self.key_evidence(&enabled);
        let mut entries: Vec<Entry> = Vec::new();
        let user = match &self.pinned {
            Some(pinned) => pinned.clone(),
            None => crate::userdict::current(),
        };
        for (rank, &id) in enabled.iter().enumerate() {
            // Earlier in the user's order wins ties.
            let tie = -(rank as f64) * 0.001;
            if !self.presses.iter().all(|&p| layout::takes(id, p)) {
                continue;
            }
            let start = entries.len();
            match id {
                QWERTY => {
                    let text = layout::qwerty(&self.presses);
                    let letters = text.chars().all(|c| c.is_ascii_alphabetic());
                    let score = if self.data.is_english(&text) || user.as_ref().is_some_and(|u| u.count(Language::English, &text) > 0) {
                        90.0
                    } else if letters && self.data.is_english_rare(&text) {
                        // A real word, just not a common one: over a reading that's no known Korean
                        // word (80), under any known Korean word (interpolate, not 인텔포라테).
                        85.0
                    } else if letters {
                        30.0
                    } else {
                        20.0
                    };
                    entries.push(Entry {
                        text,
                        score: score + tie,
                        ..Entry::default()
                    });
                }
                KANA_KOREAN | KOREAN_ROMAN => {
                    // The table's reading shows; every other pronunciation that spells a known word is
                    // offered under it, most frequent first (docs/layouts/19-kana-korean.md, "Readings").
                    let all = if id == KANA_KOREAN {
                        crate::kana_korean::readings(&self.presses, 4096)
                    } else {
                        crate::korean_roman::readings(&self.presses, 4096)
                    };
                    let Some(default) = all.first().cloned() else {
                        continue;
                    };
                    let count = self.ko_count(&user, &default);
                    let loose = default
                        .chars()
                        .any(|c| ('\u{3131}'..='\u{3163}').contains(&c));
                    let score = if count > 0 {
                        100.0 + (count as f64).ln()
                    } else if loose {
                        40.0
                    } else {
                        80.0
                    };
                    entries.push(Entry {
                        text: default,
                        score: score + tie,
                        ..Entry::default()
                    });
                    let mut known: Vec<(String, u32)> = all[1..]
                        .iter()
                        .map(|r| (r.clone(), self.ko_count(&user, r)))
                        .filter(|r| r.1 > 0)
                        .collect();
                    known.sort_by(|a, b| b.1.cmp(&a.1));
                    for (i, (text, _)) in known.into_iter().take(8).enumerate() {
                        entries.push(Entry {
                            text,
                            score: score + tie - 0.01 * (i + 1) as f64,
                        ..Entry::default()
                    });
                    }
                }
                BADA_ROMAN => {
                    // Every reading ranked by word counts: the default first only while no other
                    // reading is a better-known word (romaji shortcuts may rank first, seigong 성공);
                    // then the English dictionary, never first (docs/layouts/15-bada-roman.md).
                    let all = crate::bada_roman::readings(&self.presses, 4096, self.settings.ja_shortcuts);
                    let Some(default) = all.first().cloned() else {
                        continue;
                    };
                    let count = self.ko_count(&user, &default);
                    let loose = default.chars().any(|c| ('\u{3131}'..='\u{3163}').contains(&c));
                    // A digit or symbol with letters after it: alone, layout 15 would have ended the
                    // word there; it was staged because another layout types the key (kana's あ on 3),
                    // so this reading is unlikely (36에, not 青い).
                    let inside = (0..self.presses.len().saturating_sub(1))
                        .any(|i| crate::bada_roman::literal(&self.presses[..i], self.presses[i]).is_some());
                    let score = if inside {
                        30.0
                    } else if count > 0 {
                        100.0 + (count as f64).ln()
                    } else if loose {
                        40.0
                    } else {
                        80.0
                    };
                    // Another reading departs from the spelling (o as ㅓ, ng as ㄱ, another split): it
                    // must be much better known to rank over the reading the letters spell (jong 종,
                    // not 적; jongsok 종속, not 정석). Known still beats unknown (seigong 성공).
                    let cost = |r: &str| (BADA_ROMAN_VARIANT_COST * syllable_distance(&default, r) as f64).min(BADA_ROMAN_VARIANT_MAX);
                    let mut known: Vec<(String, f64)> = all[1..]
                        .iter()
                        .map(|r| (r, self.ko_count(&user, r)))
                        .filter(|r| r.1 > 0)
                        .map(|(r, n)| (r.clone(), 100.0 + (n as f64).ln() - cost(r)))
                        .collect();
                    known.sort_by(|a, b| b.1.total_cmp(&a.1));
                    // Nothing known: the reading that looks most like Korean (the jamo model), when
                    // it's clearly more likely than the spelling's (jiyeokchonggwalui 지역총괄의, not
                    // 지역촌꽈릐; jongsokgieobi 종속기업이).
                    let mut guess = None;
                    if count == 0 && known.is_empty() && !loose {
                        if let Some(lm) = &self.data.lm {
                            let lp = |r: &str| lm.logprob(r) - BADA_ROMAN_LM_VARIANT * syllable_distance(&default, r) as f64;
                            let base = lm.logprob(&default);
                            if let Some((r, v)) = all[1..].iter().take(512).map(|r| (r, lp(r))).max_by(|a, b| a.1.total_cmp(&b.1)) {
                                if v - base > BADA_ROMAN_LM_MARGIN {
                                    guess = Some(r.clone());
                                }
                            }
                        }
                    }
                    if let Some(g) = guess {
                        entries.push(Entry { text: g, score: score + 1.0 + tie, ..Entry::default() });
                    }
                    entries.push(Entry { text: default, score: score + tie, ..Entry::default() });
                    for (i, (text, s)) in known.into_iter().take(8).enumerate() {
                        let s = s.max(90.0);
                        entries.push(Entry { text, score: s + tie - 0.00001 * (i + 1) as f64, ..Entry::default() });
                    }
                    if let Some((word, typed, case)) = crate::bada_roman::latin_word(&self.presses) {
                        use crate::bada_roman::Case;
                        let top = entries[start..].iter().map(|e| e.score).fold(f64::MIN, f64::max);
                        // Lowercase: the common word. Any capital: brands first, then the word; all
                        // caps: brands only, the word when there's no brand by that name.
                        let names = self.data.en_ko.name(&word);
                        let dict: Vec<String> = match case {
                            Case::Lower => self.data.en_ko.word(&word),
                            Case::Caps if !names.is_empty() => names.to_vec(),
                            Case::Caps | Case::Mixed => {
                                let mut d = names.to_vec();
                                for w in self.data.en_ko.word(&word) {
                                    if !d.contains(&w) {
                                        d.push(w);
                                    }
                                }
                                d
                            }
                        };
                        for (i, text) in dict.into_iter().take(8).enumerate() {
                            entries.push(Entry {
                                text,
                                score: top - 0.5 - 0.01 * i as f64,
                                note: typed.clone(),
                                ..Entry::default()
                            });
                        }
                    }
                }
                KTS | LEGACY_KOREAN | LEGACY_MOONSHOT | BADA | K3S_JIS | BADA_PLAIN => {
                    let text = layout::compose_presses(id, &self.presses);
                    let count = self.ko_count(&user, &text);
                    let loose = text.chars().any(|c| ('\u{3131}'..='\u{3163}').contains(&c));
                    let score = if count > 0 {
                        100.0 + (count as f64).ln()
                    } else if loose {
                        40.0
                    } else {
                        80.0
                    };
                    entries.push(Entry {
                        text,
                        score: score + tie,
                        origin: Origin::Composed(id),
                        ..Entry::default()
                    });
                }
                SYMBOLS | UNICODE => {
                    // The search as typed first; what it finds under it, with names as notes.
                    let query = layout::query(&self.presses);
                    let found = if id == SYMBOLS {
                        self.data.search.symbols(&query, &self.settings.skin_tones, SEARCH_RESULTS)
                    } else {
                        self.data.search.unicode(&query, SEARCH_RESULTS)
                    };
                    let own = found.iter().find(|f| f.0 == query).map(|f| f.1.clone()).unwrap_or_default();
                    entries.push(Entry { text: query, score: 60.0 + tie, note: own, ..Entry::default() });
                    for (i, (text, note)) in found.into_iter().enumerate() {
                        entries.push(Entry { text, score: 59.0 + tie - 0.01 * i as f64, note, ..Entry::default() });
                    }
                }
                KANA | ROMAJI => {
                    let text = kana_reading(id, &self.presses);
                    let loose = text.contains(japanese::DAKUTEN)
                        || text.contains(japanese::HANDAKUTEN)
                        || text.chars().any(|c| c.is_ascii_alphabetic());
                    // The user's Japanese words (docs/dictionary-contract.md) that the reading
                    // writes, in whatever script they are: the kana typed, its katakana, one of its
                    // conversions (looked for further down than the menu lists) or a common word
                    // read so. The most used first.
                    let kata = japanese::katakana(&text);
                    let own = user.as_ref().filter(|u| u.has_positive(Language::Japanese));
                    let conversions = match &self.data.kanji {
                        Some(kanji) if !loose => kanji.convert(&text, if own.is_some() { USER_CONVERSIONS } else { CONVERSIONS }),
                        _ => Vec::new(),
                    };
                    let mut mine: Vec<(&str, u32)> = Vec::new();
                    if let Some(u) = own {
                        let forms = [text.as_str(), kata.as_str()].into_iter()
                            .chain(conversions.iter().map(|c| c.0.as_str()))
                            .chain(self.data.common_japanese(&text).iter().map(String::as_str));
                        for form in forms {
                            let count = u.count(Language::Japanese, form);
                            if count > 0 && !mine.iter().any(|m| m.0 == form) {
                                mine.push((form, count));
                            }
                        }
                        mine.sort_by(|a, b| b.1.cmp(&a.1));
                    }
                    let known = self.data.is_japanese(&text) || self.data.is_kana_native(&text) || !mine.is_empty();
                    let score = if known {
                        92.0
                    } else if loose {
                        30.0
                    } else {
                        70.0
                    };
                    // Kanji conversions (docs/spec.md §11.2), each with the kana reading as its note,
                    // above the kana itself. The kana goes where conversion ranks it (kana-native
                    // words convert to kana: この, ちょっと), a known word's no lower than second
                    // (あなた under 貴方), a kana-native word first.
                    let mut kana_score = score + tie;
                    if !loose {
                        if self.data.kanji.is_some() {
                            let mut row = 0;
                            for (conv, _) in conversions.iter().take(CONVERSIONS) {
                                let at = score + tie + 0.5 - 0.01 * row as f64;
                                if *conv == text {
                                    kana_score = kana_score.max(at + 0.001);
                                    continue;
                                }
                                // Its katakana is a row already when script forms are on (−2).
                                if self.settings.script_forms && *conv == kata {
                                    continue;
                                }
                                entries.push(Entry {
                                    text: conv.clone(),
                                    score: at,
                                    note: text.clone(),
                                    japanese: true,
                                    origin: Origin::Other,
                                    source: None,
                                });
                                row += 1;
                            }
                            // The most common words of the reading (一 for いち, 死 and 詩 for し), under
                            // the first conversion and a known word's kana (no lower than second):
                            // conversion of a lone word ranks them low.
                            for (i, word) in self.data.common_japanese(&text).iter().take(COMMON_ROWS).enumerate() {
                                entries.push(Entry {
                                    text: word.clone(),
                                    score: score + tie + 0.5 - 0.006 - 0.0001 * i as f64,
                                    note: text.clone(),
                                    japanese: true,
                                    origin: Origin::Other,
                                    source: None,
                                });
                            }
                            if known {
                                kana_score = kana_score.max(score + tie + 0.5 - 0.005);
                            }
                            if self.data.is_kana_native(&text) {
                                kana_score = kana_score.max(score + tie + 0.6);
                            }
                        }
                    }
                    // The user's own words come first, the most used at the top.
                    for (k, (word, _)) in mine.iter().enumerate() {
                        let at = score + tie + 0.7 - 0.001 * k as f64;
                        if *word == text {
                            kana_score = kana_score.max(at);
                        } else {
                            entries.push(Entry {
                                text: word.to_string(),
                                score: at,
                                note: text.clone(),
                                japanese: true,
                                origin: Origin::Other,
                                source: None,
                            });
                        }
                    }
                    entries.push(Entry::kana(text, kana_score));
                }
                _ => {}
            }
            entries[start..].iter_mut().for_each(|e| {
                e.source.get_or_insert(id);
            });
            let language = match id {
                QWERTY => Language::English,
                KANA | ROMAJI => Language::Japanese,
                _ => Language::Korean,
            };
            // In a set the key models compare the layouts (§7.0). This layout's rows move together,
            // so its own order stays: its first reading to SET_SCORE, more for a known word and
            // for a word of the user's. The evidence itself comes after the weight.
            let first = entries[start..].iter().max_by(|a, b| a.score.total_cmp(&b.score)).map(|e| (e.score, e.text.clone()));
            let evidence = evidence[rank].zip(first).map(|(evidence, (top, text))| {
                let known = if top > 84.0 { SET_KNOWN } else { 0.0 };
                let own = if user.as_ref().is_some_and(|u| u.count(language, &text) > 0) { SET_OWN } else { 0.0 };
                entries[start..].iter_mut().for_each(|e| e.score += SET_SCORE + known + own + tie - top);
                evidence
            });
            // The layout's weight (§5.3), on everything it raised.
            let weight = self.settings.weights.get(&id).copied().filter(|w| w.is_finite()).map_or(1.0, |w| w.clamp(0.0, 10.0));
            if weight != 1.0 {
                entries[start..].iter_mut().for_each(|e| e.score *= weight);
            }
            if let Some(evidence) = evidence {
                entries[start..].iter_mut().for_each(|e| e.score += evidence);
            }
            // The user's negative words (docs/dictionary-contract.md): soft ones far down, strong
            // ones gone.
            if let Some(u) = &user {
                let mut i = start;
                while i < entries.len() {
                    match u.negative(language, &entries[i].text) {
                        Some(Strength::Strong) => {
                            entries.remove(i);
                            continue;
                        }
                        Some(Strength::Soft) => entries[i].score -= 60.0,
                        None => {}
                    }
                    i += 1;
                }
            }
        }
        // Script forms (§6.4), when on: one method, at most two entries: −1 hiragana, −2 katakana.
        let method = self
            .settings
            .negative_method
            .filter(|_| self.settings.script_forms)
            .filter(|m| enabled.contains(m) && SCRIPT_FORM_METHODS.contains(m))
            .or_else(|| {
                if !self.settings.script_forms {
                    return None;
                }
                enabled
                    .iter()
                    .copied()
                    .filter(|m| SCRIPT_FORM_METHODS.contains(m))
                    .min()
            });
        if let Some(method) = method.filter(|&m| self.presses.iter().all(|&p| layout::takes(m, p)))
        {
            let hira = kana_reading(method, &self.presses);
            self.negatives = vec![Entry::kana(hira.clone(), 0.0), Entry::kana(japanese::katakana(&hira), 0.0)];
        }
        entries.extend(self.word_and_particle(&enabled));
        // One row per text, at its best score.
        entries.sort_by(|a, b| b.score.total_cmp(&a.score));
        let mut seen = std::collections::HashSet::new();
        entries.retain(|e| seen.insert(e.text.clone()));
        // Typo suggestions (docs/layouts/13-bada.md §5): right under layout 13's reading, never above it
        // and never first.
        if let Some(lm) = self.data.lm.as_ref().filter(|_| enabled.contains(&BADA)) {
            if let Some(at) = entries.iter().position(|e| e.origin == Origin::Composed(BADA)) {
                let mut n = 0;
                for (text, _) in crate::typo::suggest(lm, &self.presses, TYPO_SUGGESTIONS) {
                    let unwanted = user.as_ref().is_some_and(|u| u.negative(Language::Korean, &text) == Some(Strength::Strong));
                    if unwanted || entries.iter().any(|e| e.text == text) {
                        continue;
                    }
                    n += 1;
                    let score = entries[at].score - 0.001 * n as f64;
                    entries.insert(at + n, Entry { text, score, note: "typo?".into(), ..Entry::default() });
                }
            }
        }
        self.entries = entries;
        // After a fix (§6.4) the layout that was fixed goes on showing: its first row.
        if let Some(i) = self.stuck.and_then(|s| self.entries.iter().position(|e| e.source == Some(s))) {
            self.index = i as i32;
        }
    }

    /// Fixes what shows (§6.4): the staged keys so far keep this text, whatever is typed after
    /// them, and stay staged. The layout that read them sticks: its reading of the next keys shows.
    fn fix(&mut self) {
        if self.presses.is_empty() {
            return;
        }
        let source = if self.index >= 0 { self.entries.get(self.index as usize).and_then(|e| e.source) } else { None };
        self.prefix = self.shown();
        self.presses.clear();
        self.chosen = None;
        self.over = None;
        self.stuck = source;
        self.recompute();
    }

    fn finish(&mut self, mut out: Output) -> Output {
        if let Some(h) = &self.held {
            out.preedit = h.shown().to_string();
            out.menu = h.variants.iter().map(|c| c.to_string()).collect();
            out.selected = h.selected.unwrap_or(h.variants.len());
            out.horizontal = true;
            return out;
        }
        if let Some(pick) = &self.pick {
            out.preedit = match &pick.target {
                PickTarget::Staged { before } => format!("{before}{}", pick.rows[pick.index].0),
                _ => String::new(),
            };
            out.menu = pick
                .rows
                .iter()
                .map(|(h, c)| {
                    if c.is_empty() {
                        h.clone()
                    } else {
                        format!("{h}  {c}")
                    }
                })
                .collect();
            out.selected = pick.index;
            out.zero = 0;
            return out;
        }
        out.preedit = self.shown();
        let rows = self.negatives.len() + self.entries.len();
        if self.has_staged() && rows > 1 {
            let rows: Vec<&Entry> = self.negatives.iter().rev().chain(self.entries.iter()).collect();
            out.menu = rows.iter().map(|e| e.text.clone()).collect();
            if rows.iter().any(|e| !e.note.is_empty() || e.japanese) {
                out.notes = rows.iter().map(|e| e.note.clone()).collect();
                out.japanese = rows.iter().map(|e| e.japanese).collect();
            }
            out.zero = self.negatives.len();
            out.selected = (out.zero as i32 + self.index) as usize;
        }
        out
    }

    fn step(&mut self, forward: bool) {
        let low = -(self.negatives.len() as i32);
        let high = self.entries.len() as i32 - 1;
        // No wrap: stop at the ends (§6.4).
        self.index = if forward {
            (self.index + 1).min(high)
        } else {
            (self.index - 1).max(low)
        };
        // Remember a row picked below the top; back at the top, nothing is picked. Choosing a row
        // by hand ends the hold a layout has from an earlier fix.
        self.stuck = None;
        self.chosen = (self.index > 0)
            .then(|| self.entries.get(self.index as usize).map(|e| (e.text.clone(), e.source)))
            .flatten();
    }

    // ---------- keys ----------

    /// One key event.
    pub fn key(&mut self, key: Key, mods: Mods) -> Output {
        let mut out = match self.mode {
            Mode::Muhenkan => self.key_muhenkan(key, mods),
            Mode::Henkan => self.key_henkan(key, mods),
        };
        // The ろ key (IntlRo) is `\` and ⇧ `_`, as on every JIS keyboard (spec §6.2). A key passed
        // on is typed by macOS's ABC layout, which gives `_` for both: type it here instead.
        if key == Key::IntlRo && !out.consumed && !mods.ctrl && !mods.alt && !mods.meta {
            out.commit.push(layout::jis_char(key, mods.shift).unwrap_or('_'));
            out.consumed = true;
        }
        self.finish(out)
    }

    fn key_muhenkan(&mut self, key: Key, mods: Mods) -> Output {
        let mut out = Output::default();
        if self.held.is_some() {
            if let Some(done) = self.key_held(key, mods) {
                return done;
            }
        }
        if mods.is_shortcut() {
            return out;
        }
        // ⌥ + a letter with variants: the letter, its variants beside it, none chosen.
        if mods.alt && key.is_letter() {
            let letter = layout::jis_char(key, false).unwrap_or('?');
            if let Some(variants) = crate::hold::variants(letter, mods.shift) {
                let letter = if mods.shift { letter.to_ascii_uppercase() } else { letter };
                self.held = Some(crate::hold::Held { key, letter, variants, selected: None });
                out.consumed = true;
                return out;
            }
        }
        match key {
            Key::Space if mods.alt => {
                out.commit = "_".into();
                out.consumed = true;
            }
            Key::Backquote if !mods.alt => {
                out.need_selection = Some(if mods.shift { "half" } else { "full" }.into());
                self.waiting = out.need_selection.clone();
                out.consumed = true;
            }
            _ => {}
        }
        out
    }

    /// Keys while a variants list is open (jis-korean spec §3.6.3). None: not handled here (the
    /// list was closed by committing what showed; the key goes on as usual).
    fn key_held(&mut self, key: Key, mods: Mods) -> Option<Output> {
        let h = self.held.as_mut()?;
        let n = h.variants.len();
        let consumed = |commit: String| Output { commit, consumed: true, ..Output::default() };
        // The same ⌥ + letter again: the next variant, wrapping.
        if mods.alt && !mods.is_shortcut() && key == h.key {
            h.selected = Some(h.selected.map_or(0, |i| (i + 1) % n));
            return Some(consumed(String::new()));
        }
        if !mods.alt && !mods.is_shortcut() && !mods.shift {
            if let Some(d) = key.code().strip_prefix("Digit").and_then(|d| d.parse::<usize>().ok()) {
                let index = if d == 0 { 9 } else { d - 1 };
                let held = self.held.take()?;
                if index < n {
                    return Some(consumed(held.variants[index].to_string()));
                }
                // Past the list: the letter, and the digit goes on to the app.
                return Some(Output { commit: held.letter.to_string(), ..Output::default() });
            }
        }
        match key {
            Key::ArrowRight | Key::ArrowDown => {
                h.selected = Some(h.selected.map_or(0, |i| (i + 1) % n));
                Some(consumed(String::new()))
            }
            Key::ArrowLeft | Key::ArrowUp => {
                h.selected = Some(h.selected.map_or(n - 1, |i| (i + n - 1) % n));
                Some(consumed(String::new()))
            }
            Key::Enter => {
                let held = self.held.take()?;
                Some(consumed(held.shown().to_string()))
            }
            Key::Escape | Key::Backspace => {
                self.held = None;
                Some(consumed(String::new()))
            }
            _ => {
                // Any other key keeps what shows; then it's handled as usual (another ⌥ + letter
                // opens its own list).
                let held = self.held.take()?;
                let mut rest = self.key_muhenkan(key, mods);
                rest.commit = format!("{}{}", held.shown(), rest.commit);
                Some(rest)
            }
        }
    }

    fn commit_then_pass(&mut self) -> Output {
        Output {
            commit: self.take_shown(),
            ..Output::default()
        }
    }

    fn key_henkan(&mut self, key: Key, mods: Mods) -> Output {
        if self.pick.is_some() {
            if let Some(out) = self.key_pick(key, mods) {
                return out;
            }
        }
        if let Some(f) = key.f_index() {
            return self.f_key(f, mods);
        }
        // ⌃かな: fix what shows (§6.3).
        if key == Key::Lang1 && mods.ctrl && !mods.alt && !mods.meta && !mods.shift {
            self.fix();
            return Output { consumed: true, ..Output::default() };
        }
        if mods.is_shortcut() {
            return self.commit_then_pass();
        }
        let staged = self.has_staged();
        let consumed = |commit: String| Output {
            commit,
            consumed: true,
            ..Output::default()
        };
        match key {
            Key::Lang1 => {
                if !self.presses.is_empty() {
                    self.step(!mods.shift);
                }
                return consumed(String::new());
            }
            Key::Tab | Key::ArrowDown | Key::ArrowUp if staged && !self.presses.is_empty() => {
                let back = key == Key::ArrowUp || (key == Key::Tab && mods.shift);
                self.step(!back);
                return consumed(String::new());
            }
            Key::Enter if mods.shift && staged => return consumed(self.take_shown()),
            // Only search layouts (150, 151) in use: Space is part of the search, ⇧Space a space.
            Key::Space if !mods.alt && self.searching() => {
                if mods.shift {
                    return consumed(format!("{} ", self.take_shown()));
                }
                self.presses.push(Press { key, shift: false, alt: false });
                self.recompute();
                return consumed(String::new());
            }
            Key::Space if mods.alt => {
                let text = format!("{}\u{3000}", self.take_shown());
                return consumed(text);
            }
            Key::Backquote if !mods.alt => {
                let w = if mods.shift { Width::Half } else { Width::Full };
                if staged {
                    self.prefix = convert(&self.shown(), w);
                    self.presses.clear();
                    self.over = None;
                    self.recompute();
                    return consumed(String::new());
                }
                let mut out = consumed(String::new());
                out.need_selection = Some(if mods.shift { "half" } else { "full" }.into());
                self.waiting = out.need_selection.clone();
                return out;
            }
            Key::Backspace if staged => {
                if self.presses.pop().is_none() {
                    self.prefix.pop();
                }
                if self.presses.is_empty() {
                    self.over = None;
                }
                self.recompute();
                return consumed(String::new());
            }
            _ => {}
        }
        // bada roman: punctuation (and digits) end the word. What shows is committed first, so the
        // character can't re-rank it (한국 stays 한국 when . follows). ⌥; is ·, the one ⌥ key it takes.
        // Not when another enabled layout types the key as part of a word (kana's ぬ on 1, bada's
        // jamo on the number row): then it's staged as usual. QWERTY counts only while its own
        // reading shows, before the key and with it (It's, U.S.): then it is typing through the key.
        let candidates = self.candidates(&self.profile);
        if key.is_kana_space() && self.over.is_none() && candidates.contains(&BADA_ROMAN) {
            let press = Press { key, shift: mods.shift, alt: mods.alt };
            let others = candidates.iter().any(|&id| id != BADA_ROMAN && id != QWERTY && layout::takes(id, press));
            if let Some(c) = crate::bada_roman::literal(&self.presses, press).filter(|_| !others) {
                if self.shows(QWERTY) {
                    self.presses.push(press);
                    self.recompute();
                    if self.shows(QWERTY) {
                        return consumed(String::new());
                    }
                    self.presses.pop();
                    self.recompute();
                }
                let mut text = if self.has_staged() { self.take_shown() } else { String::new() };
                text.push(c);
                return consumed(text);
            }
        }
        if key.is_kana_space() && !(mods.alt && !mods.shift) {
            let press = Press {
                key,
                shift: mods.shift,
                alt: mods.alt,
            };
            let any = self.over.is_some() || self.candidates(&self.profile).iter().any(|&id| layout::takes(id, press));
            if any {
                // A row picked in the menu is fixed by typing on (§6.4): the new key starts after it.
                if self.chosen.is_some() {
                    self.fix();
                }
                self.presses.push(press);
                self.recompute();
                return consumed(String::new());
            }
        }
        self.commit_then_pass()
    }

    /// Keys while a hanja (or OECD) list is open. None: close it by choosing and handle the key.
    fn key_pick(&mut self, key: Key, mods: Mods) -> Option<Output> {
        let consumed = Output {
            consumed: true,
            ..Output::default()
        };
        let pick = self.pick.as_mut()?;
        match key {
            Key::ArrowDown | Key::Lang1 | Key::Tab if !(mods.shift && key != Key::ArrowDown) => {
                pick.index = (pick.index + 1).min(pick.rows.len() - 1);
                Some(consumed)
            }
            Key::ArrowUp | Key::Lang1 | Key::Tab => {
                pick.index = pick.index.saturating_sub(1);
                Some(consumed)
            }
            Key::Escape | Key::Backspace => {
                self.pick = None;
                Some(consumed)
            }
            Key::Enter | Key::Space => Some(self.choose_pick()),
            _ => {
                let mut out = self.choose_pick();
                let rest = self.key_henkan(key, mods);
                out.commit.push_str(&rest.commit);
                out.consumed = rest.consumed;
                out.need_selection = rest.need_selection;
                Some(out)
            }
        }
    }

    fn choose_pick(&mut self) -> Output {
        let Some(pick) = self.pick.take() else {
            return Output::default();
        };
        let chosen = pick.rows[pick.index].0.clone();
        let mut out = Output {
            consumed: true,
            ..Output::default()
        };
        match pick.target {
            PickTarget::Staged { before } => {
                self.clear();
                out.commit = before + &chosen;
            }
            PickTarget::Selection { before } => {
                out.commit = before + &chosen;
                out.replace_selection = true;
            }
            PickTarget::Insert => out.commit = chosen,
        }
        out
    }

    /// F6–F10 (§6.5).
    fn f_key(&mut self, f: u8, mods: Mods) -> Output {
        let pass = Output::default();
        let swallow = Output {
            consumed: true,
            ..Output::default()
        };
        // The profile's own F keys, else mac-kana's (§6.6).
        let own = self.settings.profiles.get(&self.profile).and_then(|p| p.f_keys.as_ref());
        let (intercept, keys) = match own {
            Some(f) if self.profile != PROFILES[0] => (f.intercept, &f.keys),
            _ => (self.settings.intercept_f, &self.settings.fkeys),
        };
        if !intercept {
            return pass;
        }
        let binding = keys.get(&format!("F{f}")).cloned().unwrap_or_else(|| "swallow".into());
        match binding.as_str() {
            "pass" => return pass,
            "swallow" => return swallow,
            _ => {}
        }
        // Modified F keys: up to the bound method; Bada defines none, so they're swallowed.
        if mods.shift || mods.alt || mods.ctrl || mods.meta {
            return swallow;
        }
        let Some((method, action)) = binding.split_once('.') else {
            return swallow;
        };
        let Ok(method) = method.parse::<u32>() else {
            return swallow;
        };
        if !self.candidates(&self.profile).contains(&method)
            || !ACTIONS.iter().any(|a| a.0 == method && a.1 == action)
        {
            return swallow;
        }
        self.action(method, action)
    }

    /// A method's action (docs/layouts/13-bada.md §6; Japanese: docs/spec.md §6.5).
    pub fn action(&mut self, method: u32, action: &str) -> Output {
        let mut out = Output {
            consumed: true,
            ..Output::default()
        };
        if method == LEGACY_KOREAN || method == LEGACY_MOONSHOT {
            if !self.presses.is_empty() && (action == "commit" || action == "commit-bare") {
                let text = if action == "commit" {
                    layout::compose_presses(method, &self.presses)
                } else {
                    layout::compose_bare(method, &self.presses)
                };
                out.commit = format!("{}{}", self.prefix, text);
                self.clear();
            }
            return out;
        }
        if method == BADA_ROMAN {
            if let Some(w) = match action {
                "fullwidth-latin" => Some(Width::Full),
                "halfwidth-latin" => Some(Width::Half),
                _ => None,
            } {
                // As in the Japanese layouts: the staged keys as QWERTY.
                if !self.presses.is_empty() {
                    self.over = Some(Override::Qwerty(w));
                    self.recompute();
                }
            } else if keep(action).is_some() {
                if self.has_staged() {
                    let text = self.shown();
                    self.prefix = crate::bada_roman::keep_jamo(&text, keep(action).unwrap());
                    self.presses.clear();
                    self.over = None;
                    self.recompute();
                } else {
                    out.need_selection = Some(action.to_string());
                    self.waiting = out.need_selection.clone();
                }
            }
            return out;
        }
        if method == BADA_PLAIN {
            // docs/layouts/16-bada-plain.md: bada's QWERTY actions, bada roman's jamo actions and
            // the legacy layouts' commit-bare.
            let width = match action {
                "qwerty-fullwidth" => Some(Width::Full),
                "qwerty-halfwidth" => Some(Width::Half),
                _ => None,
            };
            if let Some(w) = width {
                if !self.presses.is_empty() {
                    self.over = Some(Override::Qwerty(w));
                    self.recompute();
                }
            } else if action == "commit-bare" {
                if !self.presses.is_empty() {
                    out.commit = format!("{}{}", self.prefix, layout::compose_bare(method, &self.presses));
                    self.clear();
                }
            } else if let Some(keep) = keep(action) {
                if self.has_staged() {
                    let text = self.shown();
                    self.prefix = crate::bada_roman::keep_jamo(&text, keep);
                    self.presses.clear();
                    self.over = None;
                    self.recompute();
                } else {
                    out.need_selection = Some(action.to_string());
                    self.waiting = out.need_selection.clone();
                }
            }
            return out;
        }
        if SCRIPT_FORM_METHODS.contains(&method) {
            if !self.presses.is_empty() {
                self.over = Some(match action {
                    "hiragana" => Override::Kana {
                        method,
                        script: KanaScript::Hiragana,
                    },
                    "katakana" => Override::Kana {
                        method,
                        script: KanaScript::Katakana,
                    },
                    "halfwidth-katakana" => Override::Kana {
                        method,
                        script: KanaScript::HalfKatakana,
                    },
                    "fullwidth-latin" => Override::Qwerty(Width::Full),
                    "halfwidth-latin" => Override::Qwerty(Width::Half),
                    _ => return out,
                });
                self.recompute();
            }
            return out;
        }
        if method != BADA {
            return out;
        }
        if let Some(n) = action
            .strip_prefix("prediction-")
            .and_then(|n| n.parse::<usize>().ok())
        {
            if n >= 1 && n <= self.entries.len() && self.over.is_none() {
                self.index = (n - 1) as i32;
                out.commit = self.take_shown();
            }
            return out;
        }
        match action {
            "raw" if !self.presses.is_empty() => {
                self.over = Some(Override::Raw);
                self.recompute();
            }
            "qwerty-fullwidth" if !self.presses.is_empty() => {
                self.over = Some(Override::Qwerty(Width::Full));
                self.recompute();
            }
            "qwerty-halfwidth" if !self.presses.is_empty() => {
                self.over = Some(Override::Qwerty(Width::Half));
                self.recompute();
            }
            "hanja" => {
                if self.has_staged() {
                    let text = self.shown();
                    if let Some(pick) = self.hanja_pick(&text, true) {
                        self.pick = Some(pick);
                    }
                } else {
                    out.need_selection = Some("hanja".into());
                    self.waiting = out.need_selection.clone();
                }
            }
            _ => {}
        }
        out
    }

    fn hanja_pick(&self, text: &str, staged: bool) -> Option<Pick> {
        let whole = self.data.hanja.lookup(text);
        let (reading, candidates) = if !whole.is_empty() {
            (text, whole)
        } else {
            self.data.hanja.longest_suffix(text)?
        };
        let before = text[..text.len() - reading.len()].to_string();
        let rows = candidates
            .iter()
            .map(|c| (c.hanja.clone(), c.comment.clone()))
            .collect::<Vec<_>>();
        let target = if staged {
            PickTarget::Staged { before }
        } else {
            PickTarget::Selection { before }
        };
        Some(Pick {
            rows,
            index: 0,
            target,
        })
    }

    /// The host's selection, after an output asked for it (`need_selection`).
    pub fn host_selection(&mut self, selected: &str) -> Output {
        let mut out = Output {
            consumed: true,
            ..Output::default()
        };
        let waiting = self.waiting.take();
        match waiting.as_deref() {
            Some(w @ ("full" | "half")) if !selected.is_empty() => {
                out.commit = convert(
                    selected,
                    if w == "half" {
                        Width::Half
                    } else {
                        Width::Full
                    },
                );
                out.replace_selection = true;
            }
            Some(w) if keep(w).is_some() => {
                if !selected.is_empty() {
                    out.commit = crate::bada_roman::keep_jamo(selected, keep(w).unwrap());
                    out.replace_selection = true;
                }
            }
            Some("hanja") => {
                if !selected.is_empty() {
                    if let Some(pick) = self.hanja_pick(selected, false) {
                        self.pick = Some(pick);
                    }
                } else if !self.data.oecd.is_empty() {
                    let rows = self.data.oecd.clone();
                    self.pick = Some(Pick {
                        rows,
                        index: 0,
                        target: PickTarget::Insert,
                    });
                }
            }
            _ => {}
        }
        self.finish(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Key::*;

    /// A test engine, with no user dictionary whatever the process has loaded.
    fn engine() -> Engine {
        let data = Data::parse(
            "안녕하세요\t50\n안녕\t40\n한국\t60\n항구\t5\n아니하세요\t3\n한\t30\nㅋㅋ\t90\n감사합니다\t70\n학교\t20\n",
            "hello\nhan\n",
            "학교\t學校\t\n학\t學\t배울 학\n",
            "美\t미국\n英\t영국\n",
        );
        let mut e = Engine::new(Arc::new(data));
        e.pin_dictionary(None);
        // Bada (13) enabled: what most of these tests type on.
        e.set_settings(Settings { enabled: vec![BADA], ..Settings::default() });
        e
    }
    fn press(e: &mut Engine, k: Key) -> Output {
        e.key(k, Mods::NONE)
    }
    fn shift(e: &mut Engine, k: Key) -> Output {
        e.key(k, Mods::SHIFT)
    }

    #[test]
    fn composes_and_commits_on_space() {
        let mut e = engine();
        // 한: ㅎ(V) ㅏ(H) ㄴ(K)
        press(&mut e, KeyV);
        press(&mut e, KeyH);
        let out = press(&mut e, KeyK);
        assert_eq!(out.preedit, "한");
        assert!(out.consumed);
        let out = press(&mut e, Space);
        assert_eq!(out.commit, "한");
        assert!(!out.consumed);
        assert_eq!(out.preedit, "");
    }

    #[test]
    fn shift_enter_commits_and_swallows() {
        let mut e = engine();
        press(&mut e, KeyV);
        press(&mut e, KeyH);
        let out = e.key(Enter, Mods::SHIFT);
        assert_eq!(out.commit, "하");
        assert!(out.consumed);
        // nothing staged: the app's
        assert!(!e.key(Enter, Mods::SHIFT).consumed);
    }

    #[test]
    fn ro_key_is_backslash_and_shift_underscore() {
        // Muhenkan passes keys on; the ろ key is typed here, not by ABC (which gives `_` for both).
        let mut e = engine();
        e.set_mode(Mode::Muhenkan);
        let out = press(&mut e, IntlRo);
        assert_eq!((out.commit.as_str(), out.consumed), ("\\", true));
        assert_eq!(shift(&mut e, IntlRo).commit, "_");
        // With ⌘ or ⌃ it is the app's shortcut.
        assert!(!e.key(IntlRo, Mods { meta: true, ..Mods::NONE }).consumed);
        // Henkan: 13 types ㅢ on it; its ⇧ is the unshifted JIS character `\` and ⇧⌥ the shifted
        // one `_` (docs/layouts/13-bada.md §4).
        let mut e = engine();
        e.set_mode(Mode::Henkan);
        assert_eq!(press(&mut e, IntlRo).preedit, "ㅢ");
        assert!(shift(&mut e, IntlRo).preedit.ends_with('\\'));
        let out = e.key(IntlRo, Mods { shift: true, alt: true, ..Mods::NONE });
        assert!(format!("{}{}", out.commit, out.preedit).ends_with('_'), "{out:?}");
    }

    #[test]
    fn literal_chat_jamo_win() {
        let mut e = engine();
        press(&mut e, KeyX);
        let out = press(&mut e, KeyX);
        assert_eq!(out.preedit, "ㅋㅋ");
    }

    #[test]
    fn menu_stops_at_the_ends() {
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![BADA, QWERTY];
        e.set_settings(s);
        for k in [KeyV, KeyH, KeyK] {
            press(&mut e, k);
        }
        let n = e.entries.len();
        assert!(n > 1);
        for _ in 0..n + 3 {
            press(&mut e, ArrowDown);
        }
        assert_eq!(e.index, n as i32 - 1);
        for _ in 0..n + 3 {
            press(&mut e, ArrowUp);
        }
        assert_eq!(e.index, 0, "no negative entries in this build");
    }

    #[test]
    fn f_keys_swallowed_by_default_and_bindable() {
        let mut e = engine();
        press(&mut e, KeyV);
        let out = press(&mut e, F6);
        assert!(out.consumed);
        assert_eq!(out.preedit, "ㅎ");
        let mut s = Settings::default();
        s.enabled = vec![BADA];
        s.fkeys.insert("F7".into(), "pass".into());
        s.fkeys.insert("F8".into(), "13.raw".into());
        s.fkeys.insert("F9".into(), "13.qwerty-halfwidth".into());
        e.set_settings(s);
        assert!(!press(&mut e, F7).consumed);
        // raw: ㅎ ㅏ ㄴ stays as jamo
        let mut e2 = engine();
        e2.set_settings(e.settings().clone());
        for k in [KeyV, KeyH, KeyK] {
            press(&mut e2, k);
        }
        assert_eq!(press(&mut e2, F8).preedit, "ㅎㅏㄴ");
        let mut e3 = engine();
        e3.set_settings(e.settings().clone());
        for k in [KeyV, KeyH, KeyK] {
            press(&mut e3, k);
        }
        assert_eq!(press(&mut e3, F9).preedit, "vhk");
        let mut s = Settings::default();
        s.enabled = vec![BADA];
        s.intercept_f = false;
        e3.set_settings(s);
        assert!(!press(&mut e3, F6).consumed);
    }

    #[test]
    fn prediction_actions_commit() {
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![BADA];
        s.fkeys.insert("F6".into(), "13.prediction-1".into());
        e.set_settings(s);
        // 안녕하세요: ㅇㅏㄴ ㄴㅕㅇ ㅎㅏ ㅅㅔ ㅇㅛ
        for k in [KeyF, KeyH, KeyK, KeyE, KeyU, KeyJ, KeyV, KeyH, KeyR, KeyO, KeyF, Digit9] {
            press(&mut e, k);
        }
        let out = press(&mut e, F6);
        assert_eq!(out.commit, "안녕하세요");
    }

    #[test]
    fn hanja_staged_selection_and_oecd() {
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![BADA];
        s.fkeys.insert("F7".into(), "13.hanja".into());
        e.set_settings(s);
        // 학교: ㅎ ㅏ ㄱ ㄱ ㅛ
        for k in [KeyV, KeyH, KeyD, KeyD, Digit9] {
            press(&mut e, k);
        }
        let out = press(&mut e, F7);
        assert_eq!(out.preedit, "學校");
        let out = press(&mut e, Enter);
        assert_eq!(out.commit, "學校");
        // nothing staged: asks for the selection
        let out = press(&mut e, F7);
        assert_eq!(out.need_selection.as_deref(), Some("hanja"));
        let out = e.host_selection("학");
        assert_eq!(out.menu[0], "學  배울 학");
        let out = press(&mut e, Enter);
        assert!(out.replace_selection);
        assert_eq!(out.commit, "學");
        // no selection: OECD
        press(&mut e, F7);
        let out = e.host_selection("");
        assert_eq!(out.menu, ["美  미국", "英  영국"]);
        press(&mut e, ArrowDown);
        assert_eq!(press(&mut e, Enter).commit, "英");
    }

    #[test]
    fn japanese_layouts_and_script_forms() {
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![ROMAJI];
        s.script_forms = true;
        s.fkeys.insert("F7".into(), "21.katakana".into());
        s.fkeys.insert("F8".into(), "21.halfwidth-katakana".into());
        e.set_settings(s);
        for k in [KeyR, KeyA, Minus, KeyM, KeyE, KeyN] {
            press(&mut e, k);
        }
        let out = press(&mut e, Tab);
        assert_eq!(out.preedit, "らーめん");
        // ⇧Tab from 0: −1 hiragana, −2 katakana; then it stops
        let mut e2 = engine();
        e2.set_settings(e.settings().clone());
        for k in [KeyR, KeyA, Minus, KeyM, KeyE, KeyN] {
            press(&mut e2, k);
        }
        assert_eq!(e2.key(Tab, Mods::SHIFT).preedit, "らーめん");
        let out = e2.key(Tab, Mods::SHIFT);
        assert_eq!(out.preedit, "ラーメン");
        assert_eq!(out.menu, ["ラーメン", "らーめん", "らーめん"]);
        assert_eq!(out.zero, 2);
        assert_eq!(e2.key(Tab, Mods::SHIFT).preedit, "ラーメン");
        // F-key actions
        let mut e3 = engine();
        e3.set_settings(e.settings().clone());
        for k in [KeyR, KeyA, Minus, KeyM, KeyE, KeyN] {
            press(&mut e3, k);
        }
        assert_eq!(press(&mut e3, F7).preedit, "ラーメン");
        assert_eq!(press(&mut e3, F8).preedit, "ﾗｰﾒﾝ");
        // Kana layout: か゛ → が
        let mut e4 = engine();
        let mut s = Settings::default();
        s.enabled = vec![KANA];
        e4.set_settings(s);
        press(&mut e4, KeyT);
        assert_eq!(press(&mut e4, BracketLeft).preedit, "が");
    }

    #[test]
    fn kana_shift_option_stays_staged() {
        // ⇧⌥ + a number or symbol key on kana (20): its shifted JIS character, full-width, staged
        // with the kana (docs/layouts/20-kana.md). ⇧/ is ・, so ？ has no other key.
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![KANA];
        e.set_settings(s);
        let alt_shift = Mods { alt: true, shift: true, ..Mods::NONE };
        press(&mut e, KeyT);
        let out = e.key(Slash, alt_shift);
        assert_eq!((out.commit.as_str(), out.preedit.as_str(), out.consumed), ("", "か？", true));
        // typing goes on after it
        assert_eq!(press(&mut e, KeyT).preedit, "か？か");
        // ⌥ without ⇧ still commits what's staged and goes to the app (spec §6.3)
        let out = e.key(Slash, Mods { alt: true, ..Mods::NONE });
        assert_eq!((out.commit.as_str(), out.consumed), ("か？か", false));
        // Beside QWERTY the kana reading stays, and the key models go by the other keys (a key
        // with ⌥ is the same character in every layout): こんにちは？, not byiaf?
        let mut s = e.settings().clone();
        s.enabled = vec![QWERTY, KANA];
        e.set_settings(s);
        for k in [KeyB, KeyY, KeyI, KeyA, KeyF] {
            press(&mut e, k);
        }
        let out = e.key(Slash, alt_shift);
        assert_eq!((out.preedit.as_str(), out.menu.iter().any(|m| m == "byiaf?")), ("こんにちは？", true), "{:?}", out.menu);
    }

    #[test]
    fn henkan_profiles_link_and_switch() {
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![QWERTY, KTS, KANA];
        s.profiles.insert("henkan".into(), Profile { link: None, enabled: vec![QWERTY, KTS, ROMAJI], f_keys: None });
        // katakana: not set, so linked to mac-kana
        e.set_settings(s.clone());
        assert_eq!(e.candidates("mac-kana"), [QWERTY, KTS, KANA]);
        assert_eq!(e.candidates("henkan"), [QWERTY, KTS, ROMAJI]);
        assert_eq!(e.candidates("katakana"), [QWERTY, KTS, KANA]);
        // a link to a link; a loop falls back to mac-kana's own list
        s.profiles.insert("katakana".into(), Profile { link: Some("henkan".into()), enabled: vec![], f_keys: None });
        e.set_settings(s.clone());
        assert_eq!(e.candidates("katakana"), [QWERTY, KTS, ROMAJI]);
        s.profiles.insert("henkan".into(), Profile { link: Some("katakana".into()), enabled: vec![], f_keys: None });
        e.set_settings(s.clone());
        assert_eq!(e.candidates("henkan"), [QWERTY, KTS, KANA]);
        // mac-kana itself can link
        s.profiles.insert("henkan".into(), Profile { link: None, enabled: vec![ROMAJI], f_keys: None });
        s.profiles.insert("mac-kana".into(), Profile { link: Some("henkan".into()), enabled: vec![], f_keys: None });
        e.set_settings(s);
        assert_eq!(e.candidates("mac-kana"), [ROMAJI]);
        // switching commits what's staged and reads with the new profile's layouts
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![KANA];
        s.profiles.insert("henkan".into(), Profile { link: None, enabled: vec![ROMAJI], f_keys: None });
        e.set_settings(s);
        press(&mut e, KeyT);
        assert_eq!(e.set_profile("henkan").commit, "か");
        assert_eq!(e.profile(), "henkan");
        press(&mut e, KeyK);
        assert_eq!(press(&mut e, KeyA).preedit, "か");
        assert_eq!(e.set_profile("nonsense").commit, "か");
        assert_eq!(e.profile(), "mac-kana");
    }

    #[test]
    fn layout_weights_scale_their_entries() {
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![QWERTY, BADA];
        e.set_settings(s.clone());
        // "han" in QWERTY is a known English word: it ranks above bada's reading of the same keys
        for k in [KeyH, KeyA, KeyN] {
            press(&mut e, k);
        }
        assert_eq!(e.entries[0].text, "han");
        // bada weighted up past it
        s.weights.insert(BADA, 3.0);   // its reading starts from 80 in a set; ×3 beats the word, whatever the keys look like
        e.set_settings(s.clone());
        assert_ne!(e.entries[0].text, "han");
        // and QWERTY weighted to nothing ranks last
        s.weights.clear();
        s.weights.insert(QWERTY, 0.0);
        e.set_settings(s);
        assert_eq!(e.entries.last().unwrap().text, "han");
    }

    #[test]
    fn key_models_compare_the_layouts_of_a_set() {
        // QWERTY's `the` is bada's 판, a known word here: the keys look English, so `the` shows.
        // Bada's keys for 감사합니다 don't, so Korean shows (§7.0).
        let data = Data::parse("판\t50\n감사합니다\t70\n", "the\n", "", "");
        let mut e = Engine::new(Arc::new(data));
        e.pin_dictionary(None);
        let mut s = Settings::default();
        s.enabled = vec![QWERTY, BADA];
        e.set_settings(s);
        for k in [KeyT, KeyH, KeyE] {
            press(&mut e, k);
        }
        assert_eq!(e.shown(), "the");
        assert_eq!(e.entries[1].text, "판");
        e.flush();
        for k in [KeyD, KeyH, KeyG, KeyR, KeyH, KeyV, KeyH, KeyB, KeyE, KeyM, KeyA, KeyH] {
            press(&mut e, k);
        }
        assert_eq!(e.shown(), "감사합니다");
    }

    #[test]
    fn a_layout_alone_keeps_its_tiers() {
        // 한 is a known word: 100 + ln(count), with one layout reading the keys
        let mut e = engine();
        for k in [KeyV, KeyH, KeyK] {
            press(&mut e, k);
        }
        assert_eq!(e.entries[0].score, 100.0 + 30f64.ln());
        // ... and beside a layout that has no key model (a search)
        let mut s = Settings::default();
        s.enabled = vec![BADA, SYMBOLS];
        e.set_settings(s.clone());
        assert_eq!((e.entries[0].text.as_str(), e.entries[0].score), ("한", 100.0 + 30f64.ln()));
        // Beside QWERTY the key models compare the two: 한 starts from a known word's score in a
        // set and loses next to nothing, the keys being Korean
        s.enabled = vec![BADA, QWERTY];
        e.set_settings(s);
        let top = &e.entries[0];
        assert_eq!(top.text, "한");
        assert!(top.score <= SET_SCORE + SET_KNOWN && top.score > SET_SCORE + SET_KNOWN - 1.0, "{}", top.score);
        // ... and QWERTY's `vhk` loses no more than KEY_FLOOR
        assert!((e.entries[1].score - (SET_SCORE - KEY_FLOOR)).abs() < 0.01, "{}", e.entries[1].score);
    }

    #[test]
    fn search_layouts_take_space_as_part_of_the_search() {
        let data = Data::parse("", "", "", "").with_search(
            "$\tusd\tdollar money cash currency bill\t¥ $\n",
            "😀\tgrinning face\tface|grin|happy|smile\t\n",
            "1F600\tGRINNING FACE\n",
        );
        let mut e = Engine::new(Arc::new(data));
        e.pin_dictionary(None);
        let mut s = Settings::default();
        s.enabled = vec![SYMBOLS];
        e.set_settings(s.clone());
        for k in [KeyH, KeyA, KeyP, KeyP, KeyY, Space, KeyF, KeyA, KeyC, KeyE] {
            press(&mut e, k);
        }
        let out = press(&mut e, Tab);
        assert_eq!(out.menu, ["happy face", "😀"]);
        assert_eq!(out.preedit, "😀");
        assert_eq!(out.notes[1], "grinning face");
        // ⇧Space: what shows, then a space
        assert_eq!(e.key(Space, Mods::SHIFT).commit, "😀 ");
        // usd → $; ¥ → its currencies
        for k in [KeyU, KeyS, KeyD] {
            press(&mut e, k);
        }
        assert_eq!(press(&mut e, Tab).preedit, "$");
        press(&mut e, Escape);
        // Unicode: a code
        s.enabled = vec![UNICODE];
        e.set_settings(s);
        for k in [Digit1, KeyF, Digit6, Digit0, Digit0] {
            press(&mut e, k);
        }
        assert_eq!(press(&mut e, Tab).preedit, "😀");
        // with another layout in use, Space commits as usual
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![SYMBOLS, QWERTY];
        e.set_settings(s);
        press(&mut e, KeyA);
        assert_eq!(press(&mut e, Space).commit, "a");
    }

    #[test]
    fn f_keys_per_profile() {
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![BADA];
        s.fkeys.insert("F8".into(), "13.raw".into());
        s.profiles.insert("henkan".into(), Profile { link: Some("mac-kana".into()), enabled: vec![], f_keys: Some(FKeys { intercept: false, keys: HashMap::new() }) });
        s.profiles.insert("katakana".into(), Profile { link: Some("mac-kana".into()), enabled: vec![], f_keys: None });
        e.set_settings(s);
        // mac-kana: F8 is raw
        for k in [KeyV, KeyH, KeyK] {
            press(&mut e, k);
        }
        assert_eq!(press(&mut e, F8).preedit, "ㅎㅏㄴ");
        // henkan: its own F keys, off: F8 goes to the app
        e.set_profile("henkan");
        press(&mut e, KeyV);
        assert!(!press(&mut e, F8).consumed);
        // katakana: none of its own, so mac-kana's
        e.set_profile("katakana");
        press(&mut e, KeyV);
        assert_eq!(press(&mut e, F8).preedit, "ㅎ");
    }

    #[test]
    fn script_forms_are_off_by_default() {
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![ROMAJI];
        e.set_settings(s);
        for k in [KeyR, KeyA, Minus, KeyM, KeyE, KeyN] {
            press(&mut e, k);
        }
        let out = e.key(Tab, Mods::SHIFT);
        assert_eq!(out.preedit, "らーめん");
        assert_eq!(out.zero, 0);
        assert!(e.negatives.is_empty());
    }

    #[test]
    fn a_picked_row_stays_picked_while_typing() {
        let data = Data::parse("인터\t50\n", "inter\n#! rare\ninterpolate\n", "", "");
        let mut e = Engine::new(Arc::new(data));
        e.pin_dictionary(None);
        let mut s = Settings::default();
        s.enabled = vec![QWERTY, BADA_ROMAN];
        e.set_settings(s);
        // a rare English word: over a reading that's no Korean word
        for c in "interpolate".chars() {
            press(&mut e, Key::from_code(&format!("Key{}", c.to_ascii_uppercase())));
        }
        assert_eq!(e.shown(), "interpolate");
        e.flush();
        // 인터 is a known word: first; pick QWERTY's inter, type on: interpo… stays QWERTY's
        for c in "inteo".chars() {
            press(&mut e, Key::from_code(&format!("Key{}", c.to_ascii_uppercase())));
        }
        assert_eq!(e.shown(), "인터");
        let picked = e.entries.iter().position(|x| x.text == "inteo").unwrap();
        for _ in 0..picked {
            press(&mut e, Tab);
        }
        assert_eq!(e.shown(), "inteo");
        // typing on fixes the pick, and QWERTY goes on reading the keys after it
        press(&mut e, KeyP);
        assert_eq!(e.shown(), "inteop");
        press(&mut e, KeyO);
        assert_eq!(e.shown(), "inteopo");
        // ⌫ takes the new keys back, then the fixed text, a character at a time
        press(&mut e, Backspace);
        press(&mut e, Backspace);
        assert_eq!(e.shown(), "inteo");
        press(&mut e, Backspace);
        assert_eq!(e.shown(), "inte");
    }

    #[test]
    fn control_kana_fixes_what_shows() {
        // ⌃かな (§6.3): the keys so far keep the text that shows, staged, and its layout sticks
        let data = Data::parse("인터\t50\n", "inter\n", "", "");
        let mut e = Engine::new(Arc::new(data));
        e.pin_dictionary(None);
        let mut s = Settings::default();
        s.enabled = vec![QWERTY, BADA_ROMAN];
        e.set_settings(s);
        let ctrl = Mods { ctrl: true, ..Mods::NONE };
        // nothing staged: nothing happens, and the key is taken
        let out = e.key(Lang1, ctrl);
        assert_eq!((out.commit.as_str(), out.preedit.as_str(), out.consumed), ("", "", true));
        for c in "inteo".chars() {
            press(&mut e, Key::from_code(&format!("Key{}", c.to_ascii_uppercase())));
        }
        assert_eq!(e.shown(), "인터");
        let out = e.key(Lang1, ctrl);
        assert_eq!((out.commit.as_str(), out.preedit.as_str()), ("", "인터"));
        // what was fixed doesn't change with the keys after it; bada roman reads them
        for c in "neun".chars() {
            press(&mut e, Key::from_code(&format!("Key{}", c.to_ascii_uppercase())));
        }
        assert_eq!(e.shown(), "인터는");
        assert_eq!(press(&mut e, Space).commit, "인터는");
    }

    #[test]
    fn a_picked_entry_nothing_extends_stays_as_it_was() {
        let data = Data::parse("의\t90\n", "cockpit\n", "", "").with_en_ko("cockpit\t조종석\t콕핏\tnoun\n");
        let mut e = Engine::new(Arc::new(data));
        e.pin_dictionary(None);
        let mut s = Settings::default();
        s.enabled = vec![QWERTY, BADA_ROMAN];
        e.set_settings(s);
        for c in "cockpit".chars() {
            press(&mut e, Key::from_code(&format!("Key{}", c.to_ascii_uppercase())));
        }
        assert_eq!(e.shown(), "cockpit");
        while e.shown() != "콕핏" {
            press(&mut e, Tab);
        }
        for k in [KeyE, KeyU, KeyI] {
            press(&mut e, k);
        }
        assert_eq!(e.shown(), "콕핏의");
        assert_eq!(press(&mut e, Space).commit, "콕핏의");
    }

    #[test]
    fn bada_roman_punctuation_commits_the_word() {
        let mut e = bada_roman();
        letters(&mut e, "hanguk");
        let out = press(&mut e, Period);
        assert_eq!((out.commit.as_str(), out.preedit.as_str(), out.consumed), ("한국.", "", true));
        // after e, - is ㅐ, not punctuation
        letters(&mut e, "se");
        assert_eq!(press(&mut e, Minus).preedit, "새");
    }

    #[test]
    fn bada_roman_punctuation_stays_staged_for_kana() {
        // With kana (20) on, . is る and 3 is あ: they stay staged, and the reading with letters after
        // a digit or symbol ranks under the others
        let mut e = bada_roman();
        let mut s = e.settings().clone();
        s.enabled = vec![QWERTY, BADA_ROMAN, KANA];
        e.set_settings(s);
        letters(&mut e, "h");
        let out = press(&mut e, Period);
        assert_eq!((out.commit.as_str(), out.consumed), ("", true));
        assert!(out.menu.iter().any(|m| m == "くる"), "{:?}", out.menu);
        e.flush();
        press(&mut e, Digit3);
        press(&mut e, Digit6);
        let out = letters(&mut e, "e");
        assert_eq!(out.commit, "");
        assert_ne!(out.preedit, "36에", "{:?}", out.menu);
        // QWERTY alone beside it doesn't count: . still ends the word
        let mut s = e.settings().clone();
        s.enabled = vec![QWERTY, BADA_ROMAN];
        e.set_settings(s);
        e.flush();
        letters(&mut e, "hanguk");
        assert_eq!(press(&mut e, Period).commit, "한국.");
    }

    #[test]
    fn bada_roman_punctuation_stays_staged_while_english_shows() {
        // Beside QWERTY alone, punctuation ends layout 15's word, but not while QWERTY's reading
        // shows and still does with the key: It's is typed through
        let mut e = bada_roman();
        let mut s = e.settings().clone();
        s.enabled = vec![QWERTY, BADA_ROMAN];
        e.set_settings(s);
        letters(&mut e, "It");
        let out = shift(&mut e, Digit7);
        assert_eq!((out.commit.as_str(), out.preedit.as_str()), ("", "It'"));
        assert_eq!(letters(&mut e, "s").preedit, "It's");
        assert_eq!(press(&mut e, Space).commit, "It's");
        // a Korean word showing: the key ends it, as ever
        letters(&mut e, "hanguk");
        assert_eq!(shift(&mut e, Digit7).commit, "한국'");
    }

    #[test]
    fn bada_roman_option_semicolon_is_a_middle_dot() {
        let mut e = bada_roman();
        letters(&mut e, "saengsan");
        let out = e.key(Semicolon, Mods { alt: true, ..Mods::NONE });
        assert_eq!((out.commit.as_str(), out.consumed), ("생산·", true));
        // other ⌥ keys aren't taken
        assert!(!e.key(Comma, Mods { alt: true, ..Mods::NONE }).consumed);
    }

    #[test]
    fn bada_roman_latin_actions() {
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![BADA_ROMAN];
        e.set_settings(s);
        for k in [KeyH, KeyA, Semicolon] {
            press(&mut e, k);
        }
        e.action(BADA_ROMAN, "fullwidth-latin");
        assert_eq!(e.shown(), "ｈａ；");
        e.action(BADA_ROMAN, "halfwidth-latin");
        assert_eq!(e.shown(), "ha;");
    }

    #[test]
    fn bada_roman_number_row() {
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![BADA_ROMAN];
        e.set_settings(s);
        for k in [KeyS, KeyU, KeyL] {
            press(&mut e, k);
        }
        // a digit commits the word before it, then types itself
        let out = press(&mut e, Digit1);
        assert_eq!((out.commit.as_str(), out.preedit.as_str(), out.consumed), ("술1", "", true));
        // ⇧1 is !
        assert_eq!(shift(&mut e, Digit1).commit, "!");
    }

    #[test]
    fn kana_korean_shows_the_table_and_offers_words() {
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![KANA_KOREAN];
        e.set_settings(s);
        // は ん ぐ く: 한구쿠 shows; 한국 is offered under it
        for k in [KeyF, KeyY, KeyH, BracketLeft] {
            press(&mut e, k);
        }
        let out = press(&mut e, KeyH);
        assert_eq!(out.preedit, "한구쿠");
        assert_eq!(out.menu[..2], ["한구쿠", "한국"]);
        let out = press(&mut e, Tab);
        assert_eq!(out.preedit, "한국");
    }

    #[test]
    fn legacy_commit_actions() {
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![LEGACY_KOREAN, QWERTY];
        s.fkeys.insert("F6".into(), "11.commit-bare".into());
        s.fkeys.insert("F7".into(), "11.commit".into());
        e.set_settings(s);
        for k in [KeyR, KeyK, KeyJ] {
            press(&mut e, k);
        }
        let out = press(&mut e, F6);
        assert_eq!(out.commit, "가ㅓ");
        assert!(out.consumed);
        assert_eq!(out.preedit, "");
        // "commit" takes this layout's reading even when another ranks first (hello as 2-set)
        for k in [KeyH, KeyE, KeyL, KeyL, KeyO] {
            press(&mut e, k);
        }
        assert_eq!(press(&mut e, F7).commit, "옫이애");
    }

    #[test]
    fn bada_plain_actions() {
        // docs/layouts/16-bada-plain.md: the QWERTY actions, the jamo actions and commit-bare
        let mut e = engine();
        let mut s = Settings::default();
        assert_eq!(s.enabled, [BADA_PLAIN]);
        for (key, action) in [("F6", "initials"), ("F7", "commit-bare"), ("F8", "vowels"), ("F9", "qwerty-fullwidth"), ("F10", "qwerty-halfwidth")] {
            s.fkeys.insert(key.into(), format!("16.{action}"));
        }
        e.set_settings(s.clone());
        // 한국: ㅎ ㅏ ㄴ ㄱ ㅜ ㄱ
        let hanguk = [KeyH, KeyF, KeyL, KeyK, KeyE, KeyK];
        hanguk.iter().for_each(|&k| drop(press(&mut e, k)));
        assert_eq!(press(&mut e, F9).preedit, "ｈｆｌｋｅｋ");
        assert_eq!(press(&mut e, F10).preedit, "hflkek");
        e.flush();
        // one jamo of each syllable stays staged, and typing goes on after it
        hanguk.iter().for_each(|&k| drop(press(&mut e, k)));
        let out = press(&mut e, F6);
        assert_eq!((out.commit.as_str(), out.preedit.as_str()), ("", "ㅎㄱ"));
        assert_eq!(press(&mut e, KeyF).preedit, "ㅎㄱ아");
        e.flush();
        hanguk.iter().for_each(|&k| drop(press(&mut e, k)));
        assert_eq!(press(&mut e, F8).preedit, "ㅏㅜ");
        e.flush();
        s.fkeys.insert("F8".into(), "16.finals".into());
        e.set_settings(s);
        hanguk.iter().for_each(|&k| drop(press(&mut e, k)));
        assert_eq!(press(&mut e, F8).preedit, "ㄴㄱ");
        e.flush();
        // nothing staged: the jamo actions ask for the selection
        assert_eq!(press(&mut e, F6).need_selection.as_deref(), Some("initials"));
        assert_eq!(e.host_selection("한국").commit, "ㅎㄱ");
        // commit-bare: 가어 typed ㄱ ㅏ ㅓ commits 가ㅓ; an ㅇ asked for with ⇧ stays, so does 안
        for k in [KeyK, KeyF, KeyA] {
            press(&mut e, k);
        }
        assert_eq!(press(&mut e, F7).commit, "가ㅓ");
        press(&mut e, KeyK);
        press(&mut e, KeyF);
        shift(&mut e, KeyA);
        assert_eq!(press(&mut e, F7).commit, "가어");
        press(&mut e, KeyF);
        press(&mut e, KeyL);
        assert_eq!(press(&mut e, F7).commit, "안");
    }

    #[test]
    fn notes_for_kana_rows() {
        let mut e = engine();
        let mut s = Settings::default();
        s.enabled = vec![ROMAJI];
        s.script_forms = true;
        e.set_settings(s);
        for k in [KeyR, KeyA, Minus, KeyM, KeyE, KeyN] {
            press(&mut e, k);
        }
        let out = press(&mut e, Tab);
        assert_eq!(out.notes, ["ra-men", "ra-men", "ra-men"]);
        assert!(out.japanese.iter().all(|&j| j));
    }

    #[test]
    fn letter_variants() {
        let alt = Mods { alt: true, ..Mods::NONE };
        let alt_shift = Mods { alt: true, shift: true, ..Mods::NONE };
        let mut e = engine();
        e.set_mode(Mode::Muhenkan);
        // ⌥u: u with its variants, none chosen; 2 → ü
        let out = e.key(KeyU, alt);
        assert_eq!(out.preedit, "u");
        assert_eq!(out.menu, ["û", "ü", "ù", "ú", "ū"]);
        assert!(out.horizontal);
        assert_eq!(out.selected, 5);
        assert_eq!(press(&mut e, Digit2).commit, "ü");
        // ⇧⌥u 2 → Ü; ⌥e ⌥e ⌥e Enter → é; ⌥e ← Enter → ę
        e.key(KeyU, alt_shift);
        assert_eq!(press(&mut e, Digit2).commit, "Ü");
        for _ in 0..3 {
            e.key(KeyE, alt);
        }
        assert_eq!(press(&mut e, Enter).commit, "é");
        e.key(KeyE, alt);
        press(&mut e, ArrowLeft);
        assert_eq!(press(&mut e, Enter).commit, "ę");
        // ⌥a b → a, then b goes to the app; ⌥o Esc → nothing; a digit past the list
        e.key(KeyA, alt);
        let out = press(&mut e, KeyB);
        assert_eq!(out.commit, "a");
        assert!(!out.consumed);
        e.key(KeyO, alt);
        let out = press(&mut e, Escape);
        assert_eq!((out.commit.as_str(), out.preedit.as_str()), ("", ""));
        e.key(KeyY, alt);
        let out = press(&mut e, Digit5);
        assert_eq!(out.commit, "y");
        assert!(!out.consumed);
        // ⌥ on a letter without variants: macOS's ⌥ layer (passed on)
        assert!(!e.key(KeyB, alt).consumed);
    }

    #[test]
    fn muhenkan_passes_through() {
        let mut e = engine();
        e.set_mode(Mode::Muhenkan);
        assert!(!press(&mut e, KeyA).consumed);
        assert!(!press(&mut e, F6).consumed);
        let out = e.key(
            Space,
            Mods {
                alt: true,
                ..Mods::NONE
            },
        );
        assert_eq!(out.commit, "_");
    }

    #[test]
    fn option_space_full_width_and_hz() {
        let mut e = engine();
        press(&mut e, KeyV);
        let out = e.key(
            Space,
            Mods {
                alt: true,
                ..Mods::NONE
            },
        );
        assert_eq!(out.commit, "ㅎ\u{3000}");
        // ⇧1 is the digit; 半角/全角 makes it full-width and keeps it staged
        shift(&mut e, Digit1);
        let out = press(&mut e, Backquote);
        assert_eq!(out.preedit, "１");
        assert!(out.consumed);
    }

    /// Layout 15 alone, with a small dictionary.
    fn bada_roman() -> Engine {
        let data = Data::parse(
            "성공\t80\n세이공\t0\n사과\t50\n한국\t60\n명패\t5\n",
            "",
            "",
            "",
        )
        .with_en_ko("apple\t사과\t애플\tnoun\nagile\t\t에자일\tadj\n#! brands\nApple\t애플\ttech\nMcDonald's\t맥도날드\trestaurant\n");
        let mut e = Engine::new(Arc::new(data));
        e.pin_dictionary(None);
        let mut s = Settings::default();
        s.enabled = vec![BADA_ROMAN];
        s.fkeys.insert("F6".into(), "15.initials".into());
        s.fkeys.insert("F7".into(), "15.finals".into());
        e.set_settings(s);
        e
    }
    fn letters(e: &mut Engine, text: &str) -> Output {
        let mut out = Output::default();
        for c in text.chars() {
            let k = Key::from_code(&format!("Key{}", c.to_ascii_uppercase()));
            out = if c.is_ascii_uppercase() { shift(e, k) } else { press(e, k) };
        }
        out
    }

    #[test]
    fn bada_roman_implied_h() {
        // lt reads ㅀ + ㄷ too, at no cost: the word that's known shows first
        assert_eq!(syllable_distance("달토록", "닳도록"), 0);
        assert_eq!(syllable_distance("일어", "잃어"), 1);   // ㅀ before a vowel is still a change
        let data = Data::parse("닳도록\t40\n달도록\t30\n", "", "", "");
        let mut e = Engine::new(Arc::new(data));
        e.pin_dictionary(None);
        let mut s = Settings::default();
        s.enabled = vec![BADA_ROMAN];
        e.set_settings(s);
        assert_eq!(letters(&mut e, "daltorok").preedit, "닳도록");
    }

    #[test]
    fn bada_roman_shortcuts_may_rank_first() {
        let mut e = bada_roman();
        let out = letters(&mut e, "seigong");
        assert_eq!(out.preedit, "성공");
        // off: the letters' reading
        let mut s = e.settings().clone();
        s.ja_shortcuts = false;
        e.set_settings(s);
        assert_eq!(e.flush().commit, "세이공");
    }

    #[test]
    fn bada_roman_dictionary_never_first() {
        let mut e = bada_roman();
        let out = letters(&mut e, "apple");
        assert_ne!(out.menu[0], "사과");
        assert_eq!(out.menu[1], "사과");
        assert_eq!(out.menu[2], "애플");
        e.flush();
        let out = letters(&mut e, "Apple");
        assert_eq!(out.menu[1], "애플");
        assert_eq!(out.menu[2], "사과");
        e.flush();
        // all caps: brands only
        let out = letters(&mut e, "APPLE");
        assert_eq!(out.menu[1], "애플");
        assert!(!out.menu.contains(&"사과".to_string()));
        e.flush();
        // all caps with no brand by that name: the word
        let out = letters(&mut e, "AGILE");
        assert_eq!(out.menu[1], "에자일");
        e.flush();
        let out = letters(&mut e, "agile");
        assert_eq!(out.menu[1], "에자일");
        e.flush();
        let out = letters(&mut e, "Mcdonalds");
        assert!(out.menu.contains(&"맥도날드".to_string()));
    }

    #[test]
    fn bada_roman_keep_jamo_actions() {
        let mut e = bada_roman();
        letters(&mut e, "hanguk");
        let out = press(&mut e, F6);
        assert_eq!(out.preedit, "ㅎㄱ");
        assert_eq!(e.flush().commit, "ㅎㄱ");
        // nothing staged: the selection
        let out = press(&mut e, F7);
        assert_eq!(out.need_selection.as_deref(), Some("finals"));
        let out = e.host_selection("닭고기 abc");
        assert_eq!(out.commit, "ㄺ abc");
        assert!(out.replace_selection);
    }
}
