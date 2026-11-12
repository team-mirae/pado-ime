# Layouts

Every layout bada ime builds, one page each, and [combinations](combinations.md) of them
enabled together. Ids, kinds and names are the [spec's catalog](../spec.md) (§5).

## Summary

| Id | Layout | Display name | Kind | Reliability |
|---|---|---|---|---|
| 0 | [JIS QWERTY](00-qwerty.md) | `QWERTY` | muhenkan | English: perfect |
| 10 | [JIS KTS](10-kts.md) | `Korean (2-set)` | henkan, deterministic | Korean: perfect |
| 11 | [JIS Korean Legacy](11-korean-legacy.md) | `Korean (legacy-korean)` | henkan, deterministic | Korean: perfect |
| 12 | [JIS Moonshot](12-moonshot.md) | `Korean (legacy-moonshot)` | henkan, deterministic | Korean: perfect |
| 13 | [JIS Bada](13-bada.md) | `Korean (bada)` | henkan, deterministic | Korean: perfect |
| 14 | [JIS 3-set](14-3set-jis.md) | `Korean (3-set JIS)` | henkan, deterministic | Korean: perfect |
| 15 | [bada roman](15-bada-roman.md) | `Korean (bada roman)` | henkan, ranked | Korean: production |
| 16 | [bada plain](16-bada-plain.md) | `Korean (bada plain)` | henkan, deterministic | Korean: perfect |
| 18 | [JIS Korean Roman](18-korean-roman.md) | `Korean (Roman)` | henkan, ranked | Korean: production |
| 19 | [JIS Kana Korean](19-kana-korean.md) | `Korean (Kana)` | henkan, ranked | Korean: experimental |
| 20 | [JIS Kana](20-kana.md) | `Japanese (Kana)` | henkan, non-deterministic (kanji conversion) | Japanese (kana): production |
| 21 | [JIS Romaji](21-romaji.md) | `Japanese (Romaji)` | henkan, non-deterministic (kanji conversion) | Japanese (romaji): production |
| 150 | [Symbols](150-symbols.md) | `Symbols` | henkan, search | — |
| 151 | [Unicode](151-unicode.md) | `Unicode` | henkan, search | — |

Examples of layouts enabled together: [QWERTY + bada + kana](combinations.md#qwerty--bada--kana),
[QWERTY + bada roman + kana](combinations.md#qwerty--bada-roman--kana),
[bada + romaji](combinations.md#bada--romaji), [QWERTY + romaji](combinations.md#qwerty--romaji),
[QWERTY + KTS](combinations.md#qwerty--kts).

## Contract

What every layout provides: a page, `NNN-name.md`, and beside it a key map, `NN-name.json`.

### The page (`NNN-name.md`)

Every layout page (`NNN-name.md`) opens with a metadata header, in whatever form its maintainer
likes (the pages here use a table), giving:

- **Name:** the full canonical name in English, Korean, Japanese and the layout's native script
  language (more languages if the maintainer wants them);
- **Short name:** a short human-readable name in the same languages;
- **In-code name:** unique among all layouts (the pages here use a lowercase slug, with the id and
  the engine's constant);
- **Reliability:** for each language the layout types, the reliability its JSON claims (`perfect`,
  `production`, `experimental`; [robot.md](../robot.md#reliability)), the same as the JSON
  (`cargo test --test robot` checks);
- **Abbreviations** and shorthands (KTS for Korean 2-set);
- **Designer:** the original designer, an organization or a person;
- **Maintainer:** who maintains it now (may be the designer);
- **Contact:** a working email for the maintainer, and optionally more.

**A page is self-contained:** everything that is about its layout alone is on it, its design
principles and its history included. What several layouts share (the modes, the menu, ranking, kanji
conversion) is in the [spec](../spec.md), and their pages link to it.

The pages here are laid out alike: after the header, the board's picture (where there is one) and the
id, display name and kind, then these sections, each only where the layout has something to say:

1. **What it is**
2. **Keys and rules**: the layout in full (its keys, how it composes or reads them, its F-key actions,
   its data); [13](13-bada.md) numbers these sections §1–§6, as the source cites them
3. **Design principles**
4. **History**
5. **Open questions**
6. **Tools**: where its key map comes from, its images, how to reproduce its measurements

### Maintainers

**Ownership.** The current maintainer owns the design decisions of their layout. If they don't
respond within **90 days** to an email from an `@teammirae.com` address or a repository admin, ownership
passes to **Mirae Research**.

The layouts in this repository as of 0.1.0 are maintained by Mirae Research, designer Min Jun Sung;
where a layout builds on an earlier design (QWERTY, KS X 5002, JIS kana, 세벌식), its page names that
design too.

### The key map (`NN-name.json`)

Every layout has a key map, `NN-name.json`, in the layout editor's format (`bada-layout/1`). It is one
of three, and the layout's page says which: read by the engine as it is; the reference the engine and
the renderer follow; or written from the engine by `cargo run --example layout_json` (in `engine/`).
Only key maps are `.json` files in this folder: the typing robot and the web build read every `.json`
here.

Each file has the layout's `id` and `name`, the `keyboard` it was drawn on, and `keys`: for each of the
48 kana-space keys (by its `KeyboardEvent.code`), the tokens of its `base` and `shift` layers, each a
text `t` and a role `r`. It also says what the [typing robot](../robot.md) needs:

- `languages`: what the layout types, from `english`, `korean`, `japanese_kana`, `japanese_romaji`
  (usually one; none for a search);
- `reliability`: for each language in `languages`, what the layout claims: `perfect`, `production`
  or `experimental`. The typing robot holds it to the claim, in CI ([robot.md](../robot.md#reliability));
- `input`: how: `direct` (a key, a character), `compose` (jamo into syllables), `roman`, `kana`,
  `romaji` (spelled, then converted), `transcription` ([19](19-kana-korean.md)), `search`;
- `rules` (compose): `join_vowels`, `implied_ieung`, `double_toggle`, `held_shift`, `double_tense`;
- `spelling` (`roman`, `romaji`): the spelling the robot types for each jamo or kana;
- `transcription` (for the input of that name): the tenseness series, what ゛ ゜ っ do to an initial,
  small vowels, ー, and the readings the menu offers (vowels, finals).

Token roles: `i` initial, `f` final, `c` initial or final, `v` vowel, `a` initial with ㅏ, `n` vowel
on ㅇ, `k` kana, `l` letter, `x` text. Two tokens on one key are alternatives, chosen by the
composer's state (as on [13](13-bada.md#2-keys)'s `]`). `cargo test --test robot` checks every file against
the engine (`json_matches_engine`), and every page's Reliability row and the table above against the
files (`pages_state_the_claims`).

### Pictures

Images: `python3 tools/render/export_png.py` (HHKB JIS, light and dark, into `docs/images/layouts`),
after `python3 tools/render/build_layouts.py` when a key map changed. Interactive: open
`tools/render/index.html`.
