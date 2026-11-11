# JIS Romaji (layout 21)

<!-- layout metadata -->
| | |
|---|---|
| **Name** | English: Japanese (Romaji) · 한국어: 일본어 로마자 입력 · 日本語: ローマ字入力 |
| **Native script** | Japanese (kana) |
| **Short name** | English: Japanese (Romaji) · 한국어: 로마자 일본어 · 日本語: ローマ字 |
| **In-code name** | `romaji` (id 21, `layout::ROMAJI`) |
| **Reliability** | Japanese (romaji): **production** ([typing robot](../robot.md#reliability)) |
| **Abbreviations** | romaji |
| **Designer** | Romaji spelling as Japanese input methods use it (Hepburn and Kunrei forms). The input method: Min Jun Sung (Mirae Research) |
| **Maintainer** | Mirae Research (미래 리서치) |
| **Contact** | mj@teammirae.com (Mirae Research); issues at https://github.com/team-mirae/bada-ime |

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/layouts/21-romaji-dark.png">
  <img src="../images/layouts/21-romaji-light.png" alt="JIS Romaji (layout 21) on an HHKB JIS keyboard">
</picture>

| Id | Display name | Kind |
|---|---|---|
| 21 | `Japanese (Romaji)` | henkan, non-deterministic (kanji conversion) |

Legends: bottom of a key unshifted, top with Shift; small grey, the JIS legend. Colours: blue initial or
consonant, orange final, green vowel, purple syllable, maroon kana.

## What it is

Japanese typed in romaji, as Japanese IMEs spell it (ka か, shi / si し, tsu / tu つ, gakkou
がっこう, nn ん), with the same kanji conversion as Kana ([20](20-kana.md)).

Status: built in v0.1 (owner, 2026-11-07), with **kanji conversion** since 2026-11-07. Code: `engine/src/japanese.rs`; script forms and F-key actions in
`engine/src/engine.rs`.

## Keys and rules

The common Japanese IME spellings (`ka` か, `shi` / `si` し, `chi` / `ti` ち, `tsu` / `tu` つ, `fu` /
`hu` ふ, `ja` / `jya` / `zya` じゃ, `kya` きゃ, `fa` ふぁ, `thi` てぃ, `va` ゔぁ, small `x…` / `l…`, `xtu` っ):

- a doubled consonant is っ (`gakkou` がっこう), and so is `tch` (`matcha` まっちゃ);
- `n` before a consonant is ん; `nn` is ん, and a vowel after it starts the next kana on the second
  n (`konnichiha` and `konnnichiha` both こんにちは; `kinnyoubi` きんようび); `n'` is ん (`tan'i`
  たんい); a trailing `n` reads as ん;
- `-` ー, `,` 、, `.` 。, `/` ・, `[` 「, `]` 」; digits and other symbols full-width;
- ⇧ + a letter is that capital, full-width, as it is (Japanese IMEs switch to Latin there);
- letters still pending (`ky`) show as they are.

Shared with Kana ([20](20-kana.md)), and described in the spec ([§11](../spec.md#11-japanese-layouts-20-and-21):
ranking, kanji conversion, menu notes):

- The same menu as Kana: conversions first ([spec §11.2](../spec.md#112-kanji-conversion)), ⇧Tab for
  −1 hiragana and −2 katakana when script forms are on (off by default;
  [spec §6.4](../spec.md#64-the-suggestion-menu-negative-indexes)); the same F-key actions
  ([spec §6.5](../spec.md#65-f-key-actions)).

## Tools

Key map: `21-romaji.json`, written from the engine by `cargo run --example layout_json` (in `engine/`).
In its `spelling` table, `ん'` is ん before a vowel, y or n, `n'`.

Images: `python3 tools/render/export_png.py` (HHKB JIS). Interactive: open `tools/render/index.html`.
