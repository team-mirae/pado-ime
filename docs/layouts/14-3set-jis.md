# JIS 3-set (layout 14)

<!-- layout metadata -->
| | |
|---|---|
| **Name** | English: JIS 3-set (Sebeolsik JIS) · 한국어: 세벌식 JIS · 日本語: セボルシクJIS (JIS 3ボル式) |
| **Native script** | Korean (Hangul) |
| **Short name** | English: Korean (3-set JIS) · 한국어: 세벌식 JIS · 日本語: 3ボル式JIS |
| **In-code name** | `k3s-jis` (id 14, `layout::K3S_JIS`) |
| **Reliability** | Korean: **perfect** ([typing robot](../robot.md#reliability)) |
| **Abbreviations** | K3S-J, K3S, 3-set |
| **Designer** | 세벌식: 공병우 (Gong Byeong-u); 세벌식 순아래, the starting point: a later community design. Its JIS form: Min Jun Sung (Mirae Research) |
| **Maintainer** | Mirae Research (미래 리서치) |
| **Contact** | mj@teammirae.com (Mirae Research); issues at https://github.com/team-mirae/bada-ime |

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/layouts/14-3set-jis-dark.png">
  <img src="../images/layouts/14-3set-jis-light.png" alt="JIS 3-set (layout 14) on an HHKB JIS keyboard">
</picture>

| Id | Display name | Short | Kind |
|---|---|---|---|
| 14 | `Korean (3-set JIS)` | 세벌식 JIS, K3S-J | henkan, deterministic |

## What it is

A 3-set (세벌식) designed for JIS by the owner, from 세벌식 순아래 placed on JIS keys (the layout
editor's 3-set presets).

## Keys and rules

- **모아쓰기, natively.** A syllable's jamo can come in any order, or together as a chord: ㅏ ㅎ ㄴ,
  ㅎ ㄴ ㅏ and ㄴ ㅏ ㅎ all give 한. Each syllable has one slot for its initial, its vowel and its
  final; a jamo whose slot is taken, and that can't join what's there, starts the next syllable
  (하 ㄴ ㅏ … : ㅏ's slot is taken, so 한 then ㅏ).
- **Initials** on the right (blue), **vowels** in the middle (green), **finals** on the left and the
  number row (orange). A final is always a final and an initial always starts a syllable: nothing
  moves between syllables.
- **No jamo on ⇧.** ⇧ types the key's JIS character (⇧1 !, ⇧Q Q); `¥` is ·, `,` `.` `/` are
  themselves.
- **Tense consonants by doubling:** the same initial twice (ㄱ ㄱ → ㄲ; ㄸ ㅃ ㅆ ㅉ likewise); ㄱ or ㅅ
  twice as a final (밖).
- **Compound vowels join** (ㅗ ㅏ → ㅘ); **double finals cluster** (ㄹ ㄱ → ㄺ).

Examples: 한 M F S (or F M S, S F M) · 하나 M F H F · 까 K K F · 닭 U F W X.

## Tools

Key map: `14-3set-jis.json`, which the engine reads as it is.

Images: `python3 tools/render/export_png.py` (HHKB JIS). Interactive: open `tools/render/index.html`.
