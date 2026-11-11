//! The engine's data, loaded once by the shell and shared by every engine: the Korean word list,
//! the English word list, the hanja dictionary and the OECD menu.

use std::collections::{HashMap, HashSet};

use crate::hanja::Dictionary;

/// The Korean word list, packed: words sorted by bytes in one string, with offsets and counts.
/// Under 30 bytes a word, against ~300 in hash maps: a million words fit in some 30 MB.
#[derive(Default)]
struct KoreanWords {
    text: String,
    /// Word `i` is `text[starts[i]..starts[i + 1]]`.
    starts: Vec<u32>,
    counts: Vec<u32>,
}

impl KoreanWords {
    /// From `ko-words.tsv` lines. The file is written sorted by word bytes (build_ko_words.py), so
    /// this reads it straight in; an unsorted file is sorted first (more memory while loading).
    fn build(tsv: &str) -> KoreanWords {
        let parse = |line: &str| -> Option<(String, u32)> {
            let mut f = line.split('\t');
            let word = f.next()?;
            let count = f.next()?.trim().parse::<u32>().ok()?;
            (!word.is_empty()).then(|| (word.to_string(), count))
        };
        let lines = || tsv.lines().filter(|l| !l.starts_with('#'));
        let mut k = KoreanWords { text: String::with_capacity(tsv.len()), ..KoreanWords::default() };
        let mut sorted = true;
        let mut last = String::new();
        for line in lines() {
            let Some((w, c)) = parse(line) else { continue };
            if w.as_str() < last.as_str() {
                sorted = false;
                break;
            }
            if w == last {
                continue;
            }
            k.starts.push(k.text.len() as u32);
            k.text.push_str(&w);
            k.counts.push(c);
            last = w;
        }
        if !sorted {
            let mut all: Vec<(String, u32)> = lines().filter_map(parse).collect();
            all.sort_unstable_by(|a, b| a.0.cmp(&b.0));
            all.dedup_by(|b, a| a.0 == b.0);
            k = KoreanWords { text: String::new(), ..KoreanWords::default() };
            for (w, c) in &all {
                k.starts.push(k.text.len() as u32);
                k.text.push_str(w);
                k.counts.push(*c);
            }
        }
        k.starts.push(k.text.len() as u32);
        k.text.shrink_to_fit();
        k
    }

    fn word(&self, i: u32) -> &str {
        &self.text[self.starts[i as usize] as usize..self.starts[i as usize + 1] as usize]
    }

    fn count(&self, word: &str) -> u32 {
        let n = self.counts.len() as u32;
        let (mut lo, mut hi) = (0u32, n);
        while lo < hi {
            let mid = (lo + hi) / 2;
            match self.word(mid).cmp(word) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => return self.counts[mid as usize],
            }
        }
        0
    }
}

/// Japanese words usually written in kana (tools/build_kana_native.py), built in: small, and the
/// shell needn't ship another file.
const KANA_NATIVE: &str = include_str!("../data/ja-kana-native.txt");
/// The most common Japanese words not written in kana (tools/build_kana_native.py), built in.
const JA_COMMON: &str = include_str!("../data/ja-common-words.txt");

#[derive(Default)]
pub struct Data {
    korean: KoreanWords,
    english: HashSet<String>,
    /// Rare English words (Webster's Second, past the common list), sorted: known, but ranked under it.
    english_rare: Vec<Box<str>>,
    japanese: HashSet<String>,
    /// Japanese words usually written in kana (`ja-kana-native.txt`, built in).
    kana_native: HashSet<String>,
    /// Reading → the common words spelled with it, most common first (`ja-common-words.txt`).
    ja_common: HashMap<String, Vec<String>>,
    pub hanja: Dictionary,
    /// Kana-kanji conversion (`kanji.bin`); None without the file.
    pub kanji: Option<crate::kanji::Kanji>,
    /// The jamo 4-gram model for typo suggestions (`ko-jamo-4gram.bin`); None without the file.
    pub lm: Option<crate::typo::JamoLm>,
    /// Symbols, emoji and character names for layouts 150 and 151.
    pub search: crate::search::Search,
    /// (hanja, Korean name)
    pub oecd: Vec<(String, String)>,
    /// Layout 15's English dictionary (`en-ko.tsv`).
    pub en_ko: EnKo,
}

/// Layout 15's English dictionary (docs/layouts/15-bada-roman.md): common words and brand names.
#[derive(Default)]
pub struct EnKo {
    /// Lowercase word → (its Korean words, the word in Hangul).
    words: HashMap<String, (Vec<String>, String)>,
    /// A name's letters, lowercase → its Hangul spellings (several brands may share a name), in the
    /// order of the `#! order` section, if it lists the name.
    names: HashMap<String, Vec<String>>,
}

impl EnKo {
    /// From `en-ko.tsv`: common words (`english<TAB>korean, …<TAB>hangul<TAB>pos`), then after a
    /// `#! brands` line, brand names (`name<TAB>hangul<TAB>category`; a name with digits never
    /// matches, as only letters are looked up), then after a `#! order` line, which Hangul comes first
    /// where brands share a name (`key<TAB>hangul, hangul, …`: `engine/data/en-ko/shared-order.tsv`).
    pub fn parse(tsv: &str) -> EnKo {
        let mut e = EnKo::default();
        let mut section = "words";
        let mut order: Vec<(String, Vec<String>)> = Vec::new();
        for line in tsv.lines() {
            if let Some(rest) = line.strip_prefix("#! ") {
                section = match rest.split_whitespace().next() {
                    Some("brands") => "brands",
                    Some("order") => "order",
                    _ => section,
                };
                continue;
            }
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            match section {
                "brands" => {
                    let (Some(name), Some(hangul)) = (f.first(), f.get(1)) else { continue };
                    if name.chars().any(|c| c.is_ascii_digit()) {
                        continue;
                    }
                    let key: String = name.chars().filter(|c| c.is_ascii_alphabetic()).collect::<String>().to_ascii_lowercase();
                    if key.is_empty() || hangul.is_empty() {
                        continue;
                    }
                    let list = e.names.entry(key).or_default();
                    if !list.iter().any(|h| h == hangul) {
                        list.push(hangul.to_string());
                    }
                }
                "order" => {
                    let (Some(key), Some(list)) = (f.first(), f.get(1)) else { continue };
                    order.push((key.to_string(), list.split(',').map(|h| h.trim().to_string()).filter(|h| !h.is_empty()).collect()));
                }
                _ => {
                    let (Some(word), Some(korean), Some(hangul)) = (f.first(), f.get(1), f.get(2)) else { continue };
                    let korean: Vec<String> = korean.split(',').map(|k| k.trim().to_string()).filter(|k| !k.is_empty()).collect();
                    e.words.entry(word.to_lowercase()).or_insert((korean, hangul.to_string()));
                }
            }
        }
        // The listed spellings first, in order; any others after, as they were.
        for (key, first) in order {
            if let Some(list) = e.names.get_mut(&key) {
                let rank = |h: &String| first.iter().position(|f| f == h).unwrap_or(usize::MAX);
                list.sort_by_key(rank);
            }
        }
        e
    }

    /// A lowercase word's readings: its Korean words, then the word in Hangul.
    pub fn word(&self, word: &str) -> Vec<String> {
        self.words.get(word).map_or_else(Vec::new, |(k, h)| {
            let mut out = k.clone();
            if !h.is_empty() && !out.contains(h) {
                out.push(h.clone());
            }
            out
        })
    }

    /// A name's readings: its Hangul spellings, the first-ranked first.
    pub fn name(&self, letters: &str) -> &[String] {
        self.names.get(&letters.to_ascii_lowercase()).map_or(&[], Vec::as_slice)
    }
}

impl Data {
    /// From the files in `engine/data`: `ko-words.tsv`, `en-words.txt`, `hanja.tsv`, `oecd.tsv`.
    /// Any of them may be empty. `with_japanese` adds `ja-words.txt`.
    pub fn parse(korean: &str, english: &str, hanja: &str, oecd: &str) -> Data {
        let mut data = Data {
            hanja: Dictionary::parse(hanja),
            ..Data::default()
        };
        data.korean = KoreanWords::build(korean);
        data.kana_native = KANA_NATIVE
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| l.split('\t').next())
            .map(String::from)
            .collect();
        for line in JA_COMMON.lines().filter(|l| !l.starts_with('#')) {
            let mut f = line.split('\t');
            if let (Some(word), Some(reading)) = (f.next(), f.next()) {
                // The file is most common first, so each reading's list is too.
                data.ja_common.entry(reading.to_string()).or_default().push(word.to_string());
            }
        }
        // The list's own words, then (after `#! rare`) the rare ones: words, but not common ones.
        // One word per line; the generated part of the list has its count after a tab.
        let mut rare = false;
        for line in english.lines() {
            if line.starts_with("#! rare") {
                rare = true;
                continue;
            }
            let w = line.split('\t').next().unwrap_or("").trim();
            if w.is_empty() || w.starts_with('#') {
                continue;
            }
            if rare {
                data.english_rare.push(w.to_lowercase().into_boxed_str());
            } else {
                data.english.insert(w.to_lowercase());
            }
        }
        data.english_rare.sort_unstable();
        for line in oecd.lines().filter(|l| !l.starts_with('#')) {
            let mut f = line.split('\t');
            if let (Some(h), Some(name)) = (f.next(), f.next()) {
                data.oecd.push((h.to_string(), name.to_string()));
            }
        }
        data
    }

    /// Adds the Japanese word list (`ja-words.txt`: one word per line, `word<TAB>count` allowed).
    pub fn with_japanese(mut self, japanese: &str) -> Data {
        for line in japanese.lines().filter(|l| !l.starts_with('#')) {
            if let Some(w) = line
                .split('\t')
                .next()
                .map(str::trim)
                .filter(|w| !w.is_empty())
            {
                self.japanese.insert(w.to_string());
            }
        }
        self
    }

    /// Adds the searches of layouts 150 and 151 (`symbols.tsv`, `emoji.tsv`, `unicode-names.tsv`).
    pub fn with_search(mut self, symbols: &str, emoji: &str, unicode: &str) -> Data {
        self.search = crate::search::Search::parse(symbols, emoji, unicode);
        self
    }

    /// Adds layout 15's English dictionary (`en-ko.tsv`, built from `engine/data/en-ko/`).
    pub fn with_en_ko(mut self, tsv: &str) -> Data {
        self.en_ko = EnKo::parse(tsv);
        self
    }

    /// Adds the typo model (`ko-jamo-4gram.bin`). A malformed or missing file means no typo suggestions.
    pub fn with_lm(mut self, data: &[u8]) -> Data {
        self.lm = crate::typo::JamoLm::parse(data);
        self
    }

    /// Adds kana-kanji conversion (`kanji.bin`). A malformed file is ignored (no kanji).
    pub fn with_kanji(mut self, data: &[u8]) -> Data {
        self.kanji = if data.is_empty() { None } else { crate::kanji::Kanji::parse(data).ok() };
        if let Some(k) = &mut self.kanji {
            k.prefer_kana(self.kana_native.iter().map(String::as_str));
        }
        self
    }

    /// Whether a kana string is a Japanese word usually written in kana (この, あなた).
    pub fn is_kana_native(&self, kana: &str) -> bool {
        self.kana_native.contains(kana)
    }

    /// The common Japanese words read `kana` (written in kanji or katakana), most common first.
    pub fn common_japanese(&self, kana: &str) -> &[String] {
        self.ja_common.get(kana).map_or(&[], Vec::as_slice)
    }

    /// Whether a kana string is a known Japanese word or ends in a known word (Japanese has no spaces).
    pub fn is_japanese(&self, kana: &str) -> bool {
        self.japanese.contains(kana)
    }

    /// How often a Korean word was seen (0: never).
    pub fn korean_count(&self, word: &str) -> u32 {
        self.korean.count(word)
    }

    pub fn is_english(&self, word: &str) -> bool {
        self.english.contains(&word.to_lowercase())
    }

    /// A word of the rare English list (`#! rare` in en-words.txt: tools/build_en_rare.py).
    pub fn is_english_rare(&self, word: &str) -> bool {
        self.english_rare.binary_search_by(|w| (**w).cmp(word.to_lowercase().as_str())).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_lists() {
        let d = Data::parse(
            "안녕하세요\t50\nㅋㅋ\t9\n",
            "hello\n",
            "",
            "美\t미국\n",
        );
        assert_eq!(d.korean_count("안녕하세요"), 50);
        assert!(d.is_english("Hello"));
        assert_eq!(d.korean_count("ㅋㅋ"), 9);
        assert_eq!(d.oecd[0].0, "美");
        // en-words.txt: a count after a tab isn't part of the word; rare words after `#! rare`
        let e = Data::parse("", "hello\nlove\t1234\n#! rare\ninterpolate\n", "", "");
        assert!(e.is_english("love") && e.is_english("hello") && !e.is_english("interpolate"));
        assert!(e.is_english_rare("interpolate") && !e.is_english_rare("love"));
    }
}

#[cfg(test)]
mod en_ko_tests {
    use super::EnKo;

    #[test]
    fn brand_order_and_digits() {
        let e = EnKo::parse(
            "#! brands\nAdmiral\t아드미랄\tgaming\nAdmiral\t애드미럴\tsportswear\nA3\t에이쓰리\tcar model\nA1\t에이원\ttelecom\n#! order\nadmiral\t애드미럴, 아드미랄\n",
        );
        assert_eq!(e.name("Admiral"), ["애드미럴", "아드미랄"]);
        // names with digits never match: A doesn't offer the A3
        assert!(e.name("a").is_empty());
        // no order section: file order
        let e = EnKo::parse("#! brands\nAdmiral\t아드미랄\tgaming\nAdmiral\t애드미럴\tsportswear\n");
        assert_eq!(e.name("admiral"), ["아드미랄", "애드미럴"]);
    }
}
