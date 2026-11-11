# JIS Korean Roman (layout 18)

<!-- layout metadata -->
| | |
|---|---|
| **Name** | English: Korean (Roman) · 한국어: 로마자 한국어 입력 · 日本語: ローマ字韓国語入力 |
| **Native script** | Korean (Hangul) |
| **Short name** | English: Korean (Roman) · 한국어: 로마자 한글 · 日本語: ローマ字ハングル |
| **In-code name** | `korean-roman` (id 18, `layout::KOREAN_ROMAN`) |
| **Reliability** | Korean: **production** ([typing robot](../robot.md#reliability)) |
| **Abbreviations** | Korean Roman |
| **Designer** | Spelling: the Revised Romanization of Korean (National Institute of Korean Language, 2000). The input method: Min Jun Sung (Mirae Research) |
| **Maintainer** | Mirae Research (미래 리서치) |
| **Contact** | mj@teammirae.com (Mirae Research); issues at https://github.com/team-mirae/bada-ime |

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/layouts/18-korean-roman-dark.png">
  <img src="../images/layouts/18-korean-roman-light.png" alt="JIS Korean Roman (layout 18) on an HHKB JIS keyboard">
</picture>

| Id | Display name | Kind |
|---|---|---|
| 18 | `Korean (Roman)` | henkan, ranked |

Legends: bottom of a key unshifted, top with Shift; small grey, the JIS legend. Colours: blue initial or
consonant, orange final, green vowel, purple syllable, maroon kana.

## What it is

Korean typed as it's romanized: Revised Romanization and its common variants
(annyeonghaseyo → 안녕하세요). The **default reading** shows; every other reading of the letters is
kept, and each that spells a known Korean word is offered in the menu under it, most frequent
first (Tab / ↓ / かな; at most eight), as in Korean (Kana). bada roman ([15](15-bada-roman.md)) starts
from these spellings, and ranks the readings instead of always showing the default.

Status: built in v0.1 (owner, 2026-11-07), ranked like Korean (Kana) (19). Code:
`engine/src/korean_roman.rs`.

## Keys and rules

### The default reading

- Syllables split at the longest onset the next syllable can take: a consonant run between vowels
  leaves the next syllable the longest onset, the rest is the final (hanguk 한국, jeongmal 정말,
  annyeong 안녕). At the end of a word the whole run is the final.
- `-` or `'` splits syllables where the default wouldn't (hang-uk 항욱).
- Each spelling's first jamo in the tables below.
- Letters that don't make a syllable yet show as they are (ann → 안ㄴ, anny → 안ㄴy).
- ⇧ + a letter is the same letter; other keys type their JIS character and stay in the word
  (`hanguk1` → 한국1).

### Spellings (default first)

| | Spelling → jamo |
|---|---|
| Onsets | g ㄱ, k ㅋ/ㄱ, kk gg ㄲ, n ㄴ, d ㄷ, t ㅌ/ㄷ, tt dd ㄸ, r l ㄹ, m ㅁ, b ㅂ, p ㅍ/ㅂ, pp bb ㅃ, s ㅅ/ㅆ, ss ㅆ, sh ㅅ, j ㅈ, jj ㅉ, ch ㅊ/ㅈ, c ㅊ, h ㅎ, f ㅍ, v ㅂ, z ㅈ; a vowel alone ㅇ |
| Vowels | a ㅏ, ae ㅐ/ㅔ, ya ㅑ, yae ㅒ/ㅖ, eo ㅓ, e ㅔ/ㅐ, yeo ㅕ, ye ㅖ/ㅒ, o ㅗ/ㅓ, wa ㅘ, wae ㅙ/ㅞ/ㅚ, oe ㅚ/ㅙ/ㅞ, yo ㅛ/ㅕ, u ㅜ/ㅡ, oo ㅜ, wo weo ㅝ, we ㅞ/ㅙ/ㅚ, wi ㅟ, yu ㅠ, eu ㅡ, ui ㅢ, i ee ㅣ |
| Finals | k ㄱ/ㅋ/ㄲ, g ㄱ, kk ㄲ, n ㄴ, t ㅅ/ㄷ/ㅆ/ㅈ/ㅊ/ㅌ/ㅎ, d ㄷ, l r ㄹ, m ㅁ/ㅂ (합니다 gamsahamnida), p ㅂ/ㅍ, b ㅂ, s ㅅ, ss ㅆ, ng ㅇ/ㄱ (학년 hangnyeon); double finals lk lg ㄺ, lm ㄻ, lb lp ㄼ, lh ㅀ, ps bs ㅄ, nh ㄶ, nj ㄵ, ks ㄳ |

Other syllable splits are readings too (hanguk offers 항욱 if it were a word). Readings are searched
depth-first, the default first, up to 4,096 per segment.

## Open questions

- **[decide]** Sound changes beyond m → ㅂ and ng → ㄱ (liaison of ㅎ: joahae 좋아해; ㄴ → ㄹ:
  silla 신라) aren't readings yet.

## Tools

Key map: `18-korean-roman.json`, written from the engine by `cargo run --example layout_json` (in `engine/`).

Images: `python3 tools/render/export_png.py` (HHKB JIS). Interactive: open `tools/render/index.html`.
