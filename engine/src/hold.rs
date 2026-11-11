//! Letter variants in muhenkan mode (spec §6.2): ⌥ + a letter offers its accented and other
//! variants, macOS's press-and-hold set, as JIS English did (jis-korean docs/spec.md §3.6.3).

use crate::key::Key;

/// A letter whose variants list is open.
#[derive(Clone, Debug)]
pub struct Held {
    /// The key pressed with ⌥.
    pub key: Key,
    /// The letter, shown until a variant is chosen.
    pub letter: char,
    pub variants: &'static [char],
    /// The variant chosen with the arrows, if any; the letter shows until one is.
    pub selected: Option<usize>,
}

impl Held {
    /// What the preedit shows, and what commits if nothing else is picked.
    pub fn shown(&self) -> char {
        self.selected.map_or(self.letter, |i| self.variants[i])
    }
}

/// The variants of a letter (lower case, as typed in the mode, jis-korean spec §3.6.4), in list order
/// (numbered 1, 2, … in the list), or `None` for letters without any. macOS's English (US)
/// press-and-hold set.
pub fn variants(letter: char, shift: bool) -> Option<&'static [char]> {
    let (lower, upper): (&'static [char], &'static [char]) = match letter {
        'a' => (&['à', 'á', 'â', 'ä', 'æ', 'ã', 'å', 'ā'], &['À', 'Á', 'Â', 'Ä', 'Æ', 'Ã', 'Å', 'Ā']),
        'c' => (&['ç', 'ć', 'č'], &['Ç', 'Ć', 'Č']),
        'e' => (&['è', 'é', 'ê', 'ë', 'ē', 'ė', 'ę'], &['È', 'É', 'Ê', 'Ë', 'Ē', 'Ė', 'Ę']),
        'i' => (&['î', 'ï', 'í', 'ī', 'į', 'ì'], &['Î', 'Ï', 'Í', 'Ī', 'Į', 'Ì']),
        'l' => (&['ł'], &['Ł']),
        'n' => (&['ñ', 'ń'], &['Ñ', 'Ń']),
        'o' => (&['ô', 'ö', 'ò', 'ó', 'œ', 'ø', 'ō', 'õ'], &['Ô', 'Ö', 'Ò', 'Ó', 'Œ', 'Ø', 'Ō', 'Õ']),
        's' => (&['ß', 'ś', 'š'], &['Ś', 'Š']),
        'u' => (&['û', 'ü', 'ù', 'ú', 'ū'], &['Û', 'Ü', 'Ù', 'Ú', 'Ū']),
        'y' => (&['ÿ'], &['Ÿ']),
        'z' => (&['ž', 'ź', 'ż'], &['Ž', 'Ź', 'Ż']),
        _ => return None,
    };
    Some(if shift { upper } else { lower })
}
