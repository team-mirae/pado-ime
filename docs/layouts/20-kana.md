# JIS Kana (layout 20)

<!-- layout metadata -->
| | |
|---|---|
| **Name** | English: JIS Kana · 한국어: JIS 가나 자판 · 日本語: JISかな配列 |
| **Native script** | Japanese (kana) |
| **Short name** | English: Japanese (Kana) · 한국어: 가나 · 日本語: かな |
| **In-code name** | `kana` (id 20, `layout::KANA`) |
| **Reliability** | Japanese (kana): **production** ([typing robot](../robot.md#reliability)) |
| **Abbreviations** | JIS kana |
| **Designer** | JIS X 6002 (formerly JIS C 6233). Its bada ime form (kanji conversion, menus): Min Jun Sung (Mirae Research) |
| **Maintainer** | Mirae Research (미래 리서치) |
| **Contact** | mj@teammirae.com (Mirae Research); issues at https://github.com/team-mirae/bada-ime |

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/layouts/20-kana-dark.png">
  <img src="../images/layouts/20-kana-light.png" alt="JIS Kana (layout 20) on an HHKB JIS keyboard">
</picture>

| Id | Display name | Kind |
|---|---|---|
| 20 | `Japanese (Kana)` | henkan, non-deterministic (kanji conversion) |

Legends: bottom of a key unshifted, top with Shift; small grey, the JIS legend. Colours: blue initial or
consonant, orange final, green vowel, purple syllable, maroon kana.

## What it is

Japanese on the JIS kana legends (JIS Moonshot `A_KANA`), with kanji conversion from Mozc's
dictionary.

Status: built in v0.1 (owner, 2026-11-07), with **kanji conversion** since 2026-11-07. Code: `engine/src/japanese.rs`; script forms and F-key actions in
`engine/src/engine.rs`.

## Keys and rules

Every key types its JIS kana legend, with Shift for small kana and punctuation: ⇧3–⇧9 ぁぅぇぉゃゅょ,
⇧0 を, ⇧E ぃ, ⇧Z っ, ⇧, 、, ⇧. 。, ⇧/ ・, ⇧[ 「, ⇧] 」. ゛ (@) and ゜ ([) attach to the kana before
them (か゛ → が, は゜ → ぱ, う゛ → ゔ) and stay as they are where they can't.

Unshifted on the first line of each row, ⇧ on the second (empty: as unshifted):

| | `1` | `2` | `3` | `4` | `5` | `6` | `7` | `8` | `9` | `0` | `-` | `^` | `¥` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| key | ぬ | ふ | あ | う | え | お | や | ゆ | よ | わ | ほ | へ | ー |
| ⇧ |  |  | ぁ | ぅ | ぇ | ぉ | ゃ | ゅ | ょ | を |  |  |  |

| | `Q` | `W` | `E` | `R` | `T` | `Y` | `U` | `I` | `O` | `P` | `@` | `[` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| key | た | て | い | す | か | ん | な | に | ら | せ | ゛ | ゜ |
| ⇧ |  |  | ぃ |  |  |  |  |  |  |  |  | 「 |

| | `A` | `S` | `D` | `F` | `G` | `H` | `J` | `K` | `L` | `;` | `:` | `]` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| key | ち | と | し | は | き | く | ま | の | り | れ | け | む |
| ⇧ |  |  |  |  |  |  |  |  |  |  |  | 」 |

| | `Z` | `X` | `C` | `V` | `B` | `N` | `M` | `,` | `.` | `/` | `\` |
|---|---|---|---|---|---|---|---|---|---|---|---|
| key | つ | さ | そ | ひ | こ | み | も | ね | る | め | ろ |
| ⇧ | っ |  |  |  |  |  |  | 、 | 。 | ・ |  |

The table is JIS Moonshot's (`docs/moonshot.md` §3 there), as is the ゛ ゜ automaton.

Shared with Romaji ([21](21-romaji.md)), and described in the spec ([§11](../spec.md#11-japanese-layouts-20-and-21):
ranking, kanji conversion, menu notes):

- Typing shows the top conversion (わたしはがくせいです → 私は学生です); Tab gives the other
  conversions ([spec §11.2](../spec.md#112-kanji-conversion)); with script forms on (Settings, off by
  default), ⇧Tab **−1 hiragana**, **−2 katakana** ([spec §6.4](../spec.md#64-the-suggestion-menu-negative-indexes)).
- F-key actions: `hiragana`, `katakana`, `halfwidth-katakana`, `fullwidth-latin`, `halfwidth-latin`
  ([spec §6.5](../spec.md#65-f-key-actions)).

## Tools

Key map: `20-kana.json`, written from the engine by `cargo run --example layout_json` (in `engine/`).

Images: `python3 tools/render/export_png.py` (HHKB JIS). Interactive: open `tools/render/index.html`.
