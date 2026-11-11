//! The user dictionary (docs/dictionary-contract.md): positive words and negative words, from
//! `dictionary.json`. One per process, shared by every engine; the shell loads it.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::Deserialize;


/// How hard a negative word is pushed down.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Strength {
    /// Ranked well below what it would be; still offered.
    #[default]
    Soft,
    /// Never offered.
    Strong,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Korean,
    English,
    Japanese,
}

#[derive(Deserialize)]
struct PositiveIn {
    word: String,
    #[serde(default)]
    language: Language,
    count: Option<u32>,
    confidence: Option<String>,
}

#[derive(Deserialize)]
struct NegativeIn {
    word: String,
    #[serde(default)]
    language: Language,
    #[serde(default)]
    strength: Strength,
}

#[derive(Deserialize)]
struct FileIn {
    format: String,
    version: u32,
    #[serde(default)]
    positive: Vec<PositiveIn>,
    #[serde(default)]
    negative: Vec<NegativeIn>,
}

/// Limits (docs/dictionary-contract.md): entries beyond them are ignored.
pub const MAX_POSITIVE: usize = 20_000;
pub const MAX_NEGATIVE: usize = 20_000;

#[derive(Debug, Default)]
pub struct UserDict {
    positive: HashMap<(Language, String), u32>,
    negative: HashMap<(Language, String), Strength>,
    /// Whether it has positive words of each language (Korean, English, Japanese).
    has_positive: [bool; 3],
}

/// What `parse` found, for the shell to report.
#[derive(Debug, Default, serde::Serialize)]
pub struct Summary {
    pub ok: bool,
    pub error: String,
    pub positive: usize,
    pub negative: usize,
}

fn count_of(count: Option<u32>, confidence: Option<&str>) -> u32 {
    count.unwrap_or(match confidence {
        Some("confirmed") => 200,
        Some("likely") => 50,
        Some("tentative") => 10,
        _ => 50,
    })
}

impl UserDict {
    /// Parses `dictionary.json`. On an error, an empty dictionary and the reason.
    pub fn parse(json: &str) -> (UserDict, Summary) {
        let file: FileIn = match serde_json::from_str(json) {
            Ok(f) => f,
            Err(e) => return (UserDict::default(), Summary { error: e.to_string(), ..Summary::default() }),
        };
        if file.format != "bada-dictionary" || file.version != 1 {
            let error = format!("format must be \"bada-dictionary\", version 1 (got {:?}, {})", file.format, file.version);
            return (UserDict::default(), Summary { error, ..Summary::default() });
        }
        let mut d = UserDict::default();
        for p in file.positive.into_iter().take(MAX_POSITIVE) {
            let word = p.word.trim().to_string();
            if word.is_empty() {
                continue;
            }
            let word = if p.language == Language::English { word.to_lowercase() } else { word };
            let count = count_of(p.count, p.confidence.as_deref());
            d.has_positive[p.language as usize] = true;
            *d.positive.entry((p.language, word)).or_insert(0) += count;
        }
        for n in file.negative.into_iter().take(MAX_NEGATIVE) {
            let word = n.word.trim().to_string();
            if word.is_empty() {
                continue;
            }
            let word = if n.language == Language::English { word.to_lowercase() } else { word };
            d.negative.insert((n.language, word), n.strength);
        }
        let summary = Summary {
            ok: true,
            error: String::new(),
            positive: d.positive.len(),
            negative: d.negative.len(),
        };
        (d, summary)
    }

    pub fn count(&self, language: Language, word: &str) -> u32 {
        let key = if language == Language::English { word.to_lowercase() } else { word.to_string() };
        self.positive.get(&(language, key)).copied().unwrap_or(0)
    }

    /// Whether the dictionary has any positive word of `language` (none: nothing to look up).
    pub fn has_positive(&self, language: Language) -> bool {
        self.has_positive[language as usize]
    }

    pub fn negative(&self, language: Language, word: &str) -> Option<Strength> {
        let key = if language == Language::English { word.to_lowercase() } else { word.to_string() };
        self.negative.get(&(language, key)).copied()
    }

}

static CURRENT: RwLock<Option<Arc<UserDict>>> = RwLock::new(None);

/// Replaces the process's dictionary (an empty string clears it). Returns what was loaded.
pub fn load(json: &str) -> Summary {
    if json.trim().is_empty() {
        *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = None;
        return Summary { ok: true, ..Summary::default() };
    }
    let (dict, summary) = UserDict::parse(json);
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = if summary.ok { Some(Arc::new(dict)) } else { None };
    summary
}

/// The process's dictionary, if any.
pub fn current() -> Option<Arc<UserDict>> {
    CURRENT.read().unwrap_or_else(|e| e.into_inner()).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_contract() {
        let (d, s) = UserDict::parse(r#"{
            "format": "bada-dictionary", "version": 1,
            "positive": [{"word": "데이터센터", "count": 30}, {"word": "Kubernetes", "language": "english"},
                         {"word": "サーバー", "language": "japanese", "confidence": "confirmed"}],
            "negative": [{"word": "감소합니다", "strength": "strong"}, {"word": "god", "language": "english"}],
            "unknown_field": true
        }"#);
        assert!(s.ok, "{}", s.error);
        assert_eq!((s.positive, s.negative), (3, 2));
        assert_eq!(d.count(Language::Korean, "데이터센터"), 30);
        assert_eq!(d.count(Language::English, "kubernetes"), 50);
        assert_eq!(d.count(Language::Japanese, "サーバー"), 200);
        assert!(d.has_positive(Language::Japanese) && d.has_positive(Language::English));
        assert!(!UserDict::parse(r#"{"format": "bada-dictionary", "version": 1, "positive": [{"word": "가"}]}"#).0.has_positive(Language::Japanese));
        assert_eq!(d.negative(Language::Korean, "감소합니다"), Some(Strength::Strong));
        assert_eq!(d.negative(Language::English, "God"), Some(Strength::Soft));
        let (_, bad) = UserDict::parse(r#"{"format": "other", "version": 1}"#);
        assert!(!bad.ok);
    }
}
