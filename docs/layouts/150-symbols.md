# Symbols (layout 150)

<!-- layout metadata -->
| | |
|---|---|
| **Name** | English: Symbols · 한국어: 기호 · 日本語: 記号 |
| **Native script** | (none: symbols and emoji) |
| **Short name** | English: Symbols · 한국어: 기호 · 日本語: 記号 |
| **In-code name** | `symbols` (id 150, `layout::SYMBOLS`) |
| **Reliability** | none: a search, it declares no language |
| **Abbreviations** | — |
| **Designer** | Min Jun Sung (Mirae Research). Emoji names: Unicode CLDR |
| **Maintainer** | Mirae Research (미래 리서치) |
| **Contact** | mj@teammirae.com (Mirae Research); issues at https://github.com/team-mirae/bada-ime |

| Id | Display name | Kind |
|---|---|---|
| 150 | `Symbols` | henkan, search (Mirae Research range 150–199) |

## What it is

A search. What you type stays first; what it finds is listed under it (at most 40), each with its
canonical name in the menu's note column. Tab or ↓ moves to a result.

## Keys and rules

- **Letters and digits** spell the search, either case. **Space** adds a word (`happy face`);
  **⇧Space** commits what shows and types a space. (Space searches only when every layout in use is
  a search layout, 150 or 151; with others enabled it commits as usual.)
- **A symbol** offers its richer forms: `.` → 。 … ‥ · • ・ ．, `[` → 「 『 【, `¥` → the currencies.
- **A single letter or digit** offers its variants: accented letters, full-width, circled;
  superscript, subscript and roman numerals for digits.
- **Words** search symbols by canonical name and tags, and emoji by name and CLDR keywords: every
  word must begin a word of the name or tags. Name matches come before tag matches.
- **Emoji with skin tones** are offered in the order set in Settings (none, light, medium-light,
  medium, medium-dark, dark by default); a tone left out isn't offered.

### Data

- `engine/data/symbols.tsv`: the symbols, hand-written: text, **canonical name**, tags, and the keys
  that offer it. Currency names are their ISO codes (`usd` → $), and `$` is also tagged dollar,
  money, cash, currency, bill. The names are bada ime's own proposal, for review.
- **Currencies:** the top 30 economies by GDP per capita (World Bank, 2023), one currency each with
  duplicates merged (EUR once for France, Germany, Italy …): EUR, CHF, BMD, KYD, NOK, GBP, SGD, USD,
  ISK, QAR, DKK, MOP, AUD, SEK, CAD, ILS, HKD; plus KRW and JPY (the owner's). A currency with a sign
  of its own shows it ($ € ₩ ¥ £ ₪), else its local short form (S$, HK$) or its code (CHF, NOK).
- `engine/data/emoji.tsv`: emoji 18.0 names and CLDR English keywords, built by
  `engine/tools/build_unicode_data.py` (Unicode License v3).

## Tools

Key map: `150-symbols.json`, written from the engine by `cargo run --example layout_json` (in `engine/`).
