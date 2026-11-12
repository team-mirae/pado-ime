# bada roman (layout 15)

<!-- layout metadata -->
| | |
|---|---|
| **Name** | English: bada roman · 한국어: 바다 로마자 · 日本語: バダローマ字 |
| **Native script** | Korean (Hangul) |
| **Short name** | English: bada roman · 한국어: 바다 Roman · 日本語: バダRomaji |
| **In-code name** | `bada-roman` (id 15, `layout::BADA_ROMAN`) |
| **Reliability** | Korean: **production** ([typing robot](../robot.md#reliability)) |
| **Abbreviations** | bada roman |
| **Designer** | Min Jun Sung (Mirae Research) |
| **Maintainer** | Mirae Research (미래 리서치) |
| **Contact** | mj@teammirae.com (Mirae Research); issues at https://github.com/team-mirae/bada-ime |

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/layouts/15-bada-roman-dark.png">
  <img src="../images/layouts/15-bada-roman-light.png" alt="bada roman (layout 15) on an HHKB JIS keyboard">
</picture>

| Id | Display name | Kind |
|---|---|---|
| 15 | `Korean (bada roman)` | henkan, ranked |

## What it is

Korean typed as it's romanized (`hanguk` 한국, `annyeonghaseyo` 안녕하세요), on the letters as QWERTY
has them. It starts from the spellings of Korean Roman ([18](18-korean-roman.md)) and adds keys of its
own (`q`, `x`, `;`, `-` after `e`), informal spellings (`woo`, `young`, `sung`), an English
dictionary (`apple` offers 사과) and Japanese on'yomi as shortcuts (`mei` 명). Every reading of the
letters is kept and ranked by word counts: what the letters spell shows, unless another reading is a
far better-known word. Digits and punctuation type themselves and end the word.

Built (`engine/src/bada_roman.rs`; ranking in `engine/src/engine.rs`). The number and symbol keys not
named below type their JIS characters (owner, 2026-11-11).

## Keys and rules

- **An English dictionary.** English words typed in letters convert to Korean. The case says
  which readings (owner, 2026-11-11):
  - lowercase: the common word only: the Korean word first, then the word written in Hangul as it
    sounds: `apple` 사과, then 애플;
  - any capital (`Apple`, `iPhone`, `McDonalds`, a single `A`): brand names too, first, then the
    common word: `Apple` 애플, then 사과;
  - all caps (two letters or more, all capitals): a brand is more likely, so brand names only:
    `APPLE` 애플. With no brand by that name, the common word: `AGILE` 에자일.

  **A dictionary reading never ranks first: at best it's second.** Whatever the letters type as
  Korean comes first; `apple` offers 사과 at row 2 and 애플 at row 3, `Apple` 애플 at row 2 and 사과
  at row 3. At most eight dictionary rows are offered, each with the word as typed as its note.
  Only letters are looked up, so a name with digits (Audi A3, 007) never matches.

  **Brands sharing a name** (Corona: a Mexican beer and a Colombian ceramics maker) offer each
  Hangul spelling; the one that shows first is the brand with the most revenue, where that's known;
  else the most consumer-facing; else the shortest Hangul; else the one whose country code comes
  first alphabetically (owner, 2026-11-11). Revenue and consumer calls:
  `engine/data/en-ko/shared-rank.tsv`; the order the engine reads, with the other two rules applied:
  `shared-order.tsv`, from `engine/tools/build_shared_order.py`.

  **Initialisms** are written as the brand writes itself in Korean, where it does (BTS 방탄소년단,
  SSG 쓱, TGV 테제베, CCTV 씨씨티비); otherwise with the Korean names of the English letters, whatever
  the brand's country (owner, 2026-11-11): USA 유에스에이, SNCF 에스엔씨에프 (not 에스엔세에프), ARD
  에이알디. C is 씨 and Z 제트. A name said as a word is written as it's said: UEFA 유에파, IKEA 이케아.

  **Nouns, and borrowed words.** The dictionary keeps nouns, offering their noun senses only
  (`red` 빨강, not 빨갛다). A verb, adjective or adverb is kept only when Korean borrows it as it
  is, and then it offers only the word in Hangul, not nominalized (no -하다, no -ing form):
  `agile` 에자일, `sprint` 스프린트, `brief` 브리프. Other words are set aside in
  `engine/data/en-ko/non-nouns.tsv`, not built.

  US spelling (`color`, not `colour`). The Hangul is the spelling Koreans have established for the word,
  official or not (foil 호일, python 파이썬, app 앱, `Apple` 애플). The standard loanword spelling
  (외래어 표기법) isn't used. A word with no established spelling is written as it sounds, with
  English short a (/æ/) as ㅔ, never ㅐ: `agile` 에자일. Data:
  `engine/data/en-ko/` (being built): common words (things, actions, descriptions) and brand names
  from every country. Vulgar, hateful and sensitive words are kept apart in `sensitive.tsv`, left
  out of builds; `make SENSITIVE=1` (in `mac/`) builds them in.

- **`q` is `ch` (ㅊ)**: `qa` 차, `qeong` 청.
- **`zz` is ㅉ**, as `jj` is (`z` is ㅈ, as `j` is): `zza` 짜, `zzigae` 찌개 (owner, 2026-11-11).
- **`woo` is ㅜ** (as Woori writes it): `woori` 우리.
- **`you` is ㅠ, and ㅕ before a final** (as names write it: Young 영): `you` 유, `young` 영.
- **`eui` is ㅢ** too (as names and older romanizations write it): `eui` 의, `gaeui` 개의.
- **Ranking: the spelling first.** A reading that reads a letter as another jamo (`o` ㅓ, `ng` ㄱ,
  `u` ㅡ …) must be far better known to rank over what the letters spell: about 20 times as
  frequent per jamo it changes (`jong` 종 before 적, `jongsok` 종속 before 정석). Splitting the
  letters elsewhere costs nothing (a silent ㅇ aside: 기어비 / 기업이 are the same jamo). A known word
  still ranks over an unknown spelling (`seigong` 성공). When no reading is a known word, the jamo
  model (`ko-jamo-4gram.bin`) picks the one that looks most Korean, at the same kind of cost per
  changed jamo: `jiyeokchonggwalui` 지역총괄의, not 지역촌꽈릐. On the robot's corpora this raised
  first-row hits from 93.4 / 97.0 / 95.9% to 96.3 / 99.9 / 97.8% (Common Voice / common words / law).
- **Punctuation ends the word.** A key that types a character as it is (the digits, `.` `,` `/` and
  the other symbol keys, with or without ⇧; `-` where it doesn't lengthen) commits what shows, then
  the character, so it can't change the reading before it: `hanguk` then `.` commits 한국 and the
  full stop. Not when another enabled layout, QWERTY aside, types
  that key as part of a word (kana's る on `.`, あ on `3`; bada's jamo on the number row): then
  the key is staged as usual, and a bada roman reading with letters after such a key ranks under
  the others (`3 6 e` shows 青い, not 36에). Beside QWERTY the key is staged too while QWERTY's
  reading is the one that shows, before the key and with it: `It's`, `U.S.` and `Hello,` are typed
  through as English. With a Korean reading showing, the key ends the word as above (`don't` comes
  out 돈't: 돈 is what shows when `'` arrives).
- **`ts` is a final ㅆ** (`t`, a final ㅅ, and `s` again): `its` 있; before a vowel the default
  gives `s` to the next syllable (`itseo` 잇서) and 있어, a known word, ranks first.
- **`h` is a final ㅎ** too: `joh` 좋. Before a vowel the default gives `h` to the next syllable
  (`joha` 조하), and 좋아, a known word, ranks first.
- **`l` before `t` implies a ㅎ** between them (owner, 2026-11-11): `lt` also reads ㅀ + ㄷ, as
  it's said (ㅎ + ㄷ sounds like ㅌ): `daltorok` 닳도록, `silta` 싫다, `ilta` 잃다. The ranking
  takes ㅀ + ㄷ for the same letters as ㄹ + ㅌ, so the known word comes first. `lh` before a
  vowel stays as it was (`ilheo` 잃어).
- **`j` and `z` are a final ㅈ** too, as `t` is (owner, 2026-11-11): `naj` 낮, `jaj` 잦. Before a
  vowel the default gives them to the next syllable (`naja` 나자), and 낮아, a known word, ranks
  first. `nj` is still ㄵ (`anj` 앉).
- **`;` separates syllables**: `han;guk` 한국, `hang;uk` 항욱. `-` and `'` don't (unlike Korean
  Roman, 18).
- **`-` lengthens ㅔ, ㅖ and ㅞ** into ㅐ, ㅒ and ㅙ, as ー does ㅔ and ㅖ in Korean (Kana), 19:
  `e-` 애, `se-` 새, `he-` 해, `ye-` 얘, `we-` 왜, `dwe-` 돼 (ㅞ: owner, 2026-11-11). The other vowels
  have no long form; after them, and anywhere else, `-` is a hyphen (`a-` 아-).
- **`x` is ㅇ as an initial**, as Korean chat writes it (ㅇㅇ, ㅇㅋ): `xx` ㅇㅇ, `xk` ㅇㅋ. Before
  a vowel it starts a syllable with ㅇ, which also splits: `anxa` 안아, never 아나. `x` is in no
  romanization, so it reads nothing else.
- **Number and symbol keys: JIS, with ₩ and 「」** (owner, 2026-11-11). Each key types its JIS
  characters on both layers, with these exceptions: `¥` is ₩, ⇧`0` is `+`, ⇧`[` 「 and ⇧`]` 」
  (full-width), and `;` is the syllable break (⇧`;` types `;`):

  | | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 0 | - | ^ | ¥ | @ | [ | : | ] | , | . | / | \ | ; |
  |---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
  | base | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 0 | - | ^ | ₩ | @ | [ | : | ] | , | . | / | \ | break |
  | ⇧ | ! | " | # | $ | % | & | ' | ( | ) | + | = | ~ | \| | ` | 「 | * | 」 | < | > | ? | _ | ; |

  ⌥`;` is the middle dot `·` (U+00B7, 가운뎃점); other ⌥ keys aren't taken. ⇧⌥ + a number or symbol
  key types its shifted JIS character, as in bada (13). `-` after `e` still lengthens it (`e-` 애). Each of these keys commits what shows before typing
  its character (Punctuation ends the word, above).

- **Interlaced vowels: `yu` and `u` before a final.** Common informal spellings write ㅕ as `yu`
  and ㅓ as `u` when a final follows (Hyundai 현대, Kyung 경, Sung 성, Jung 정), so there a
  spelling reads two vowels, as `]` types two jamo in bada (13) by where it falls:
  - `yu` + a final reads ㅕ as well as ㅠ: `nyun` 년, `hyun` 현, `kyung` 경. ㅕ is the usual one
    (55 to 3, syllable counts with ㄴ ㅇ ㄹ ㅁ ㄱ ㅂ finals);
  - `u` + a final reads ㅓ as well as ㅜ and ㅡ: `sung` 성, `jun` 전. Here the counts are close
    (ㅓ 80, ㅜ 52, ㅡ 120), so the word decides.

  At the end of a syllable they keep their one reading: `yu` 유, `nyu` 뉴, `su` 수. Which reading
  shows is ranked like any other reading.
- **Japanese romaji readings as shortcuts.** A Sino-Korean syllable can be typed as the on'yomi of
  its character, when that's shorter: `mei` 명 (明, myeong), `jou` 정 (定, jeong), `nen` 년 (年,
  nyeon). Unlike the English dictionary, a shortcut **may rank first**, over what the letters type
  as Korean. A romaji with several syllables offers them most frequent first (`jou` 정, 성, 증…).
  **Shortcuts mix with Korean spelling inside a word**, syllable by syllable, with every other
  rule of this layout: `seigong` 성공 (sei 성 + gong 공), `meipe-` 명패 (mei 명 + pe- 패),
  `kenkang` 건강. Each syllable may be either; the readings that make known words are offered,
  ranked like any others.
  The shortcuts are the characters of both Korea's education hanja and Japan's jōyō kanji whose
  readings are close or alike (similarity 0.85 or more), whose Korean reading takes 4 or more
  keys, and whose romaji takes fewer (counting ㅕ before a final as `yu`: 연 is `yun`, 3 keys,
  so `en` 연 isn't one): `engine/data/ja-shortcuts.tsv` (160, from
  `python3 engine/tools/build_ja_shortcuts.py`; the scoring is in the script).

  A setting, on by default: **Accept Japanese romaji readings as shortcuts when it saves
  keystrokes**, with the tooltip "i.e. mei as 명, or jou as 정".

### F-key actions

Bound to nothing by default (spec §6.5). The jamo actions (`initials`, `vowels`, `finals`) act on the
**staged text** (the result stays staged in its place, and typing goes on after it); with nothing
staged, on the **selection**; with no selection, on nothing.
Only Hangul syllables change: everything else, Latin, digits, punctuation, lone jamo, stays as it
is, in place.

| Action id | What it does | Example |
|---|---|---|
| `initials` | Leaves only each syllable's initial consonant | 한국 → ㅎㄱ, 안녕! → ㅇㄴ! |
| `vowels` | Leaves only each syllable's vowel (a compound vowel stays whole) | 한국 → ㅏㅜ, 과자 → ㅘㅏ |
| `finals` | Leaves only each syllable's final (a double final stays whole); a syllable with no final is deleted | 한국 → ㄴㄱ, 닭고기 → ㄺ, 가나 → (nothing) |
| `fullwidth-latin` | The staged keys as QWERTY, full-width (as in the Japanese layouts, 20 and 21). Lasts until the commit | `ha;` → ｈａ； |
| `halfwidth-latin` | The staged keys as QWERTY, half-width | `ha;` → ha; |

## History

Why it exists, in the owner's words: the [README](../../README.md#bada-roman).

- Tried and taken out (2026-11-11): `,` ㅓ, `.` ㅡ, `:` ㅢ, `@` final ㅇ, `0` `oo`, `9` `yo`, `8`
  `yu`; then the number and symbol keys typed nothing on the base layer, until they went back to JIS.
- Decided against (owner, 2026-11-11): a second reading for informal name spellings (`oo` ㅠ after
  y as in Yoon, `uh` ㅓ as in Suh, `ah`, a silent final `h`, `oi` ㅚ, `ar` ㅏ, initial `l` as ㅇ or
  ㄴ). Readings stay rule-based.

## Open questions

- **[decide]** Names of several words (`Coca-Cola`, `General Motors`) and inflected forms
  (`apples`, `ran`).

## Tools

Key map: `15-bada-roman.json`, written from the engine by `cargo run --example layout_json` (in
`engine/`). Data, each built by a script in `engine/tools/`: the romaji shortcuts
(`build_ja_shortcuts.py`), which Hangul shows first where brands share a name
(`build_shared_order.py`), the rare English words QWERTY ranks beside it (`build_en_rare.py`). The
robot types it in Revised Romanization from the key map's `spelling` table, with `;` as the break
([robot.md](../robot.md)); the English dictionary and the romaji shortcuts aren't typed.

Images: `python3 tools/render/export_png.py` (HHKB JIS). Interactive: open `tools/render/index.html`.
