# JIS KTS (layout 10)

<!-- layout metadata -->
| | |
|---|---|
| **Name** | English: Korean 2-set (KS X 5002), on JIS · 한국어: 두벌식 표준 자판 (KS X 5002), JIS 배열 · 日本語: 韓国語2ボル式 (KS X 5002)、JIS配列 |
| **Native script** | Korean (Hangul) |
| **Short name** | English: Korean (2-set) · 한국어: 두벌식 · 日本語: 2ボル式 |
| **In-code name** | `kts` (id 10, `layout::KTS`) |
| **Reliability** | Korean: **perfect** ([typing robot](../robot.md#reliability)) |
| **Abbreviations** | KTS, 2-set, 2벌식 |
| **Designer** | The Korean national standard KS X 5002 (formerly KS C 5715, 1982). Its JIS form in bada ime: Min Jun Sung (Mirae Research) |
| **Maintainer** | Mirae Research (미래 리서치) |
| **Contact** | mj@teammirae.com (Mirae Research); issues at https://github.com/team-mirae/bada-ime |

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/layouts/10-kts-dark.png">
  <img src="../images/layouts/10-kts-light.png" alt="JIS KTS (layout 10) on an HHKB JIS keyboard">
</picture>

| Id | Display name | Kind |
|---|---|---|
| 10 | `Korean (2-set)` | henkan, deterministic |

Legends: bottom of a key unshifted, top with Shift; small grey, the JIS legend. Colours: blue initial or
consonant, orange final, green vowel, purple syllable, maroon kana.

## What it is

The standard Korean 2-set (두벌식, KS X 5002) on the JIS letter keys. The number row and
symbol keys keep their JIS characters.

It's Korea's national standard as it is, so there is no design of bada ime's own here, and no
history but the standard's: what a 2-set typist's hands know carries over key for key. Only its
place on a JIS board is bada ime's.

## Keys and rules

- ⇧ gives the tense consonants ㅃ ㅉ ㄸ ㄲ ㅆ (Q W E R T) and ㅒ ㅖ (O P); every other ⇧ letter
  types the same jamo.
- Composition is the usual 2-set one: consonants and vowels join into syllables, a consonant
  followed by a vowel moves to the next syllable, double finals from two keys (ㄴ ㅎ → ㄶ).
- With QWERTY (0) enabled alongside it, English and Korean come from one mode
  ([QWERTY + KTS](combinations.md#qwerty--kts)).

## Tools

Key map: `10-kts.json`, written from the engine by `cargo run --example layout_json` (in `engine/`).

Images: `python3 tools/render/export_png.py` (HHKB JIS). Interactive: open `tools/render/index.html`.
