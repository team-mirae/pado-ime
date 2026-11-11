# The typing robot

`engine/tests/robot.rs` types real text on every layout, as a person would, and fails when a layout
can't type its own language well. It is the test for "can this layout type Korean (English,
Japanese)?", on law texts and everyday sentences.

    cd engine && cargo test --release --test robot -- --nocapture

About half a minute; release builds only (`#[cfg_attr(debug_assertions, ignore)]`: in debug it takes minutes). The report, with failing examples:
`engine/target/robot-report.md`. `ROBOT_LIMIT=n` types only the first n distinct items of each corpus.

## What it knows

Only the layout's JSON (`docs/layouts/NN-name.json`, see [layouts/README.md](layouts/README.md)):
which language it declares, how it's typed (`input`), what each key's tokens are, its rules and
spelling tables. It plans the keys from that alone, presses them on a fresh engine with only that
layout enabled, and reads the screen: what was committed and the marked text, and the menu.

A second test, `json_matches_engine`, checks every key of every file against the engine, so the
robot's plans are the layouts' real keys. The searches (150, 151) declare no language, so they have
key maps and no corpora.

## How it types

- **direct** (QWERTY, English): every character needs a key. Muhenkan sends keys to the app as
  they are, so nothing more is checked.
- **compose** (10–14, Korean): syllable by syllable. For each syllable it lists the ways to type it
  from the tokens: the initial (`i`/`c`, or nothing where ㅇ is implied), the vowel (`v`, or two that
  join: ㅗ ㅏ), the final (`f`/`c`, or two that cluster: ㄹ ㄱ; the same key twice where the layout
  doubles), and whole-syllable keys (`a`, `n`). Fewest presses first, unshifted first. It keeps up to
  four plans whose screen shows the word so far, exactly; a person checks each syllable as it
  appears. A layout must type **every** word.
- **roman** (15, 18): the word in the file's spelling table (Revised Romanization, letter by
  letter), syllables run together, then split by the file's break character if that fails (`-` in
  18, `;` in 15).
- **kana** (20): each kana on its key; a voiced kana as its plain kana and ゛ (゜).
- **romaji** (21): the reading in the spelling table, two kana at once where the table has them;
  っ doubles the next consonant, ん is nn, or `n'` before a vowel, y or n (`zen'in` ぜんいん: after
  `nn` a vowel starts the next kana, [layouts/21-romaji.md](layouts/21-romaji.md)).
- **transcription** (19): from the file's `transcription` map. Each syllable starts from a kana
  key's syllable (た 타); its initial is moved along the **tenseness series** (ㄱ/ㅋ/ㄲ, ㄷ/ㅌ/ㄸ,
  ㅂ/ㅍ/ㅃ, ㅈ/ㅊ/ㅉ, ㅅ/ㅆ) by the modifiers: っ before the kana raises it to tense, ゛ after it lowers
  aspirated to lax (and ㅅ ㅈ, ㅎ ㅂ, ㅇ ㅂ), ゜ makes ㅎ ㅍ; its vowel changed by a small vowel
  (replacing, or joining: う ⇧ぇ 웨) or by ー (ㅔ ㅐ); its final from a final key (ん ⇧ん ⇧む つ). Plans
  that need a **reading** the menu offers rather than shows (ㅜ as ㅡ, ㅗ as ㅓ, ク as a final ㄱ,
  plain ん as ㅇ, っ as a final ㅅ) cost more. Plans that need none are checked syllable by syllable
  against the preedit; the whole word must show or be in the first 5 rows (the menu offers a
  reading only when it spells a known word). At most 80 screen checks per word.

For converting layouts (15, 18, 19, 20, 21) the wanted text must show, or be among the first **5**
rows of the menu. The report lists, per layout and corpus, the syllables it couldn't type, most frequent
first.

## Reliability

Each layout claims a **reliability** for each language it declares (`reliability` in its JSON), and
the robot fails if the layout doesn't earn it:

| Reliability | Bar |
|---|---|
| `perfect` | every item of every corpus of the language |
| `production` | every one of the language's 1,000 most common words, and at least **80%** of each other corpus |
| `experimental` | none: typed and reported, never fails |

The report's **Claims** table gives each layout's claim and what it earns. Each layout page states
its claims in its metadata header, and the layouts README in its table; `pages_state_the_claims`
(a fast test, also in debug builds) checks they match the JSON. A layout may claim less
than it earns, never more; claiming less is a **warning** (on stderr, in the report, and as an
annotation on the GitHub Actions run), not a failure. GitHub Actions (`.github/workflows/robot.yml`) runs the robot on every
push to main and every pull request, and puts the report in the run's summary.

| Layout | Language | Claims |
|---|---|---|
| 00 | english | perfect |
| 10–14 | korean | perfect |
| 15 | korean | production (99.7% or more of each corpus, 96–99.9% in the first row; the English dictionary and romaji shortcuts aren't typed) |
| 18 | korean | production (99.5% or more of each corpus; 63–83% in the first row, the rest from the menu) |
| 19 | korean | experimental (69% on Common Voice; the gaps are open questions in [layouts/19-kana-korean.md](layouts/19-kana-korean.md#open-questions)) |
| 20, 21 | japanese_kana, japanese_romaji | production (every common word; 87.5% of the law texts, 93% of Tatoeba) |

## How a word fails

- **Untypeable:** no plan shows it (the report gives the syllable and what showed instead).
- **Commit mid-word:** it only comes out right typed as two pieces, a commit between them.
- **Not in the first 5 rows** (converting layouts): what row 0 showed.

## Corpora

`engine/tests/data/robot/`, sampled to 400 sentences each by `engine/tools/build_robot_corpus.py`
(sources and licences in [SOURCES.md](../engine/tests/data/robot/SOURCES.md)):

| Language | Files |
|---|---|
| English | `en-us-law.txt` (17 U.S.C.), `en-uk-law.txt` (Human Rights Act 1998), `en-tatoeba.txt` |
| Korean | `ko-law.txt` (대한민국헌법, 민법), `ko-common-voice.txt` |
| Japanese | `ja-law.tsv` (日本国憲法, 民法), `ja-tatoeba.tsv`: one phrase per line with its reading |

and each language's 1,000 most common words, for `production`: `en-common.txt` (Tatoeba),
`ko-common.txt` (the counts in `engine/data/ko-words.tsv`), `ja-common.tsv` (Tatoeba, content words
in their dictionary form, with readings; 私 read わたし, words with digits left out).

English is typed word by word (letters, digits, apostrophes), Korean by runs of Hangul, Japanese by
phrase (a content word and the particles and auxiliaries after it, read by SudachiPy). Everything
else in a line (punctuation, Latin and digits in Korean) is counted and left out.
