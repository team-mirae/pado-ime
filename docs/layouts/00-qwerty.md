# JIS QWERTY (layout 0)

<!-- layout metadata -->
| | |
|---|---|
| **Name** | English: JIS QWERTY · 한국어: JIS 쿼티 · 日本語: JIS QWERTY配列 |
| **Native script** | English (Latin) |
| **Short name** | English: QWERTY · 한국어: 쿼티 · 日本語: QWERTY |
| **In-code name** | `qwerty` (id 0, `layout::QWERTY`) |
| **Reliability** | English: **perfect** ([typing robot](../robot.md#reliability)) |
| **Abbreviations** | QWERTY |
| **Designer** | QWERTY: Christopher Latham Sholes (1870s); its JIS keyboard form: JIS X 6002. Its bada ime form: Min Jun Sung (Mirae Research) |
| **Maintainer** | Mirae Research (미래 리서치) |
| **Contact** | mj@teammirae.com (Mirae Research); issues at https://github.com/team-mirae/bada-ime |

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/layouts/00-qwerty-dark.png">
  <img src="../images/layouts/00-qwerty-light.png" alt="JIS QWERTY (layout 0) on an HHKB JIS keyboard">
</picture>

| Id | Display name | Kind |
|---|---|---|
| 0 | `QWERTY` | muhenkan |

Legends: bottom of a key unshifted, top with Shift; small grey, the JIS legend. Colours: blue initial or
consonant, orange final, green vowel, purple syllable, maroon kana.

## What it is

The JIS English layout: every key types its JIS legend. It's the layout of **muhenkan mode**
(英数), where nothing composes or stages ([spec §4.2](../spec.md)). Ported from JIS Korean / JIS
Moonshot `A_QWERTY`.

## Keys and rules

- The keys go to the app as they are, typed through the keyboard layout bada ime sets in muhenkan
  (macOS's ABC, as a JIS keyboard has it). The ろ key is the one the engine types itself: `\`, and
  `_` with ⇧ (ABC gives `_` for both).
- ⌥ + a letter shows its accented variants (⌥e: è é ê ë ē ė ę), picked with 1–9, 0 or the arrows;
  ⌥Space types `_` ([spec §6.2](../spec.md)).
- 半角/全角 makes the selection full-width; ⇧半角/全角 half-width.
- Caps Lock on types by the muhenkan rule: ALL CAPS, or UPPER_SNAKE (Space types `_`)
  ([spec §6.7](../spec.md)).
- It can be a **henkan candidate** too: enabled in henkan mode, its plain reading (the letters as
  typed) is ranked with the other layouts' readings: a common English word over a Korean reading
  that's no known word, a rare one too, any known Korean word over both
  ([spec §6.4](../spec.md)). That's how English words come out of henkan mode without switching
  ([combinations](combinations.md)).

## Tools

Key map: `00-qwerty.json`, written from the engine by `cargo run --example layout_json` (in `engine/`).

Images: `python3 tools/render/export_png.py` (HHKB JIS). Interactive: open `tools/render/index.html`.
