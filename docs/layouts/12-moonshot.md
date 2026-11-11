# JIS Moonshot (layout 12)

<!-- layout metadata -->
| | |
|---|---|
| **Name** | English: JIS Moonshot · 한국어: JIS 문샷 · 日本語: JIS ムーンショット |
| **Native script** | Korean (Hangul) |
| **Short name** | English: Korean (legacy-moonshot) · 한국어: 문샷 · 日本語: ムーンショット |
| **In-code name** | `legacy-moonshot` (id 12, `layout::LEGACY_MOONSHOT`) |
| **Reliability** | Korean: **perfect** ([typing robot](../robot.md#reliability)) |
| **Abbreviations** | Moonshot, legacy-moonshot |
| **Designer** | Min Jun Sung (Mirae Research), from the JIS Moonshot input method (Mode A) |
| **Maintainer** | Mirae Research (미래 리서치) |
| **Contact** | mj@teammirae.com (Mirae Research); issues at https://github.com/team-mirae/bada-ime |

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/layouts/12-moonshot-dark.png">
  <img src="../images/layouts/12-moonshot-light.png" alt="JIS Moonshot (layout 12) on an HHKB JIS keyboard">
</picture>

| Id | Display name | Kind |
|---|---|---|
| 12 | `Korean (legacy-moonshot)` | henkan, deterministic |

Legends: bottom of a key unshifted, top with Shift; small grey, the JIS legend. Colours: blue initial or
consonant, orange final, green vowel, purple syllable, maroon kana.

## What it is

The Korean reading of **JIS Moonshot Mode A** (`~/Projects/jis-moonshot`, moonshot.md §9),
without its predictions.

## Keys and rules

- **Letters:** 2-set. ⇧ + consonant adds ㅏ (⇧ㄱ → 가); ⇧ + vowel puts it on ㅇ (⇧ㅗ → 오; ⇧O 애 and
  ⇧P 에 too).
- Tense consonants on 1–5 (ㅃ ㅉ ㄸ ㄲ ㅆ), compound vowels on 6–0 (ㅟ ㅝ ㅚ ㅢ ㅘ); ⇧ + a number key
  is its digit.
- ㅙ ㅖ ㅒ ㅢ on `-` `;` `:` `,`; with ⇧ the first three start a syllable on ㅇ (왜 예 얘), and ⇧`,`
  is a comma. The other symbol keys type their JIS characters (⇧`.` is a full stop too).
- **Implied ㅇ** and the same vowel key twice (마리 → 말이), as in [11](11-korean-legacy.md); no Shift
  held over (⇧ㄱ ⇧ㅏ → 가아).
- Enabling **0, 12, 20** together reproduces JIS Moonshot: QWERTY, Moonshot Korean and JIS kana read
  at once ([spec §4.3](../spec.md)), ranked by this engine's rules, not Moonshot's predictor.
- Like 11, it publishes `commit` and `commit-bare`.

## Tools

Key map: `12-moonshot.json`, written from the engine by `cargo run --example layout_json` (in `engine/`).

Images: `python3 tools/render/export_png.py` (HHKB JIS). Interactive: open `tools/render/index.html`.
