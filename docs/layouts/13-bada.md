# JIS Bada (layout 13)

<!-- layout metadata -->
| | |
|---|---|
| **Name** | English: JIS Bada · 한국어: JIS 바다 레이아웃 · 日本語: JIS バダレイアウト |
| **Native script** | Korean (Hangul) |
| **Short name** | English: bada · 한국어: 바다 · 日本語: バダ |
| **In-code name** | `bada` (id 13, `layout::BADA`) |
| **Reliability** | Korean: **perfect** ([typing robot](../robot.md#reliability)) |
| **Abbreviations** | bada |
| **Designer** | Min Jun Sung (Mirae Research) |
| **Maintainer** | Mirae Research (미래 리서치) |
| **Contact** | mj@teammirae.com (Mirae Research); issues at https://github.com/team-mirae/bada-ime |

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/layouts/13-bada-dark.png">
  <img src="../images/layouts/13-bada-light.png" alt="JIS Bada (layout 13) on an HHKB JIS keyboard">
</picture>

| Id | Display name | Kind |
|---|---|---|
| 13 | `Korean (bada)` | henkan, deterministic |

Legends: bottom of a key unshifted, top with Shift; small grey, the JIS legend. Colours: blue initial or
initial-or-final, orange (underlined) final only, green vowel. Q is ㅃ, initial only. `]` is an interlaced key: ㅃ as
an initial, ㅄ (small, orange, at the right) as a final, chosen by where the key falls in the
syllable, not by Shift.

§1–§6 are the layout as built; the source cites them by number (`docs/layouts/13-bada.md §5`). After
them: [Design principles](#design-principles), the [History](#history) (its sections have numbers of
their own, §H1–§H17), [Open questions](#open-questions) and [Tools](#tools).

## 1. What it is

bada ime's own Korean layout, for JIS keyboards. It's **deterministic**: the same keys always give
the same text, nothing is guessed. The key map's source is `docs/layouts/13-bada.json`; the
engine is `engine/src/layout.rs` (keys and composing) and `engine/src/typo.rs` (typo suggestions).
How it came to be: [History](#history).

**bada types like 2-set, keeps finals like 3-set, and uses the whole key space like JIS kana.**
(두벌식처럼 치고, 세벌식처럼 받침을 지키고, JIS 가나처럼 키보드 전체를 쓴다.) It takes how each works,
not where its keys are: KTS habits don't carry over key for key.

| From | What bada takes | What it doesn't |
|---|---|---|
| **KTS (2-set)** | Consonants on the left, vowels on the right, so the hands alternate. Consonant keys that are a final until a vowel takes them. Vowels that join (ㅗ ㅏ → ㅘ). | KTS's key positions. Shift for tense consonants and ㅒ ㅖ. |
| **3-set (세벌식)** | Final-only keys (J K L ; ¥ and their ⇧ double finals) that never move to the next syllable. Every compound vowel on a key of its own. | 3-set's sides (initials right, finals left there). A final-only key for every final. |
| **JIS kana** | All 48 kana-space keys carry letters, the number row included, and ⇧ on number and symbol keys is kept for digits and symbols, as JIS kana keeps it for its punctuation. | Anything kana-specific. |

Bada was the default henkan layout until bada plain ([16](16-bada-plain.md)), a candidate to replace
it, became the default (owner, 2026-11-12). Its usual companions are QWERTY and kana
([QWERTY + bada + kana](combinations.md#qwerty--bada--kana)) or romaji
([bada + romaji](combinations.md#bada--romaji)).

## 2. Keys

Unshifted on the first line of each row, ⇧ on the second (empty: as unshifted). ᶠ marks a
final-only key; a consonant without it is an initial, or a final until a vowel takes it (§3).

| | `1` | `2` | `3` | `4` | `5` | `6` | `7` | `8` | `9` | `0` | `-` | `^` | `¥` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| key | ㅒ | ㅆ | ㅉ | ㄲ | ㄸ | ㅟ | ㅝ | ㅠ | ㅛ | ㅘ | ㅙ | ㅞ | ㅂᶠ |
| ⇧ | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 0 | - | ^ | ㄼᶠ |

| | `Q` | `W` | `E` | `R` | `T` | `Y` | `U` | `I` | `O` | `P` | `@` | `[` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| key | ㅃ | ㅌ | ㄴ | ㅅ | ㅍ | ㅚ | ㅕ | ㅑ | ㅔ | ㅐ | ㅖ | ㅒ |
| ⇧ |  |  |  |  |  |  |  |  |  |  | @ | [ |

| | `A` | `S` | `D` | `F` | `G` | `H` | `J` | `K` | `L` | `;` | `:` | `]` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| key | ㄷ | ㅈ | ㄱ | ㅇ | ㅁ | ㅏ | ㅇᶠ | ㄴᶠ | ㄹᶠ | ㄱᶠ | ㅆ | ㅃ ㅄᶠ |
| ⇧ |  |  |  |  |  |  | ㅁᶠ | ㄶᶠ | ㄺᶠ | ㄲᶠ | : | ] |

| | `Z` | `X` | `C` | `V` | `B` | `N` | `M` | `,` | `.` | `/` | `\` |
|---|---|---|---|---|---|---|---|---|---|---|---|
| key | ㅊ | ㅋ | ㄹ | ㅎ | ㅂ | ㅓ | ㅣ | ㅡ | ㅗ | ㅜ | ㅢ |
| ⇧ |  |  |  |  |  |  |  | , | . | / | \ |

- **Initial-only:** F ㅇ, Q ㅃ, 3 ㅉ, 5 ㄸ (ㅉ and ㄸ are never finals; Q is ㅃ even after a vowel:
  아 Q ㅏ → 아빠).
- **Interlaced:** `]` types ㅃ as an initial and ㅄ as a final, by the composer's state (after a
  vowel: ㅄ, 값; otherwise ㅃ), not by Shift. A vowel after ㅄ takes ㅃ: 값 ㅏ → 가빠.
- **Initial-or-final:** W E R T, A S D G, Z X C V B, 2 4 and `:`.
- **Final-only:** J ㅇ, K ㄴ, L ㄹ, `;` ㄱ, ¥ ㅂ, the five commonest finals; with ⇧, ㅁ ㄶ ㄺ ㄲ ㄼ.
- **Vowels:** all 21 have a key: the simple ones on the right half and the bottom row, the compound
  ones and ㅒ ㅖ on the number row and the keys right of P.

## 3. Composing

- **ㅇ is typed** (F). A vowel with nothing before it is the bare vowel (ㅏ).
- **A consonant after a vowel is its final.** An initial-or-final key stays the final until the
  next key is a vowel, which takes it as its initial, as in 2-set: 가 E → 간, 가 E ㅏ → 가나.
- **A final-only key never moves:** 가 K ㅏ → 간ㅏ. Use E for a ㄴ that starts the next syllable.
- **Double finals:** two finals in a row join when they make one (ㄹ ㅁ → ㄻ, ㄴ ㅎ → ㄶ); a vowel
  then takes the second as its initial if its key can be one. Or one ⇧ key: ⇧J ㅁ, ⇧K ㄶ, ⇧L ㄺ,
  ⇧; ㄲ, ⇧¥ ㄼ.
- **Vowels join as in 2-set** (ㅗ ㅏ → ㅘ, ㅡ ㅣ → ㅢ), though every compound vowel has its key too.
- **Consonants alone stay as typed** (ㄱㅅ, ㅋㅋ).

Examples: 한 V H K · 안녕 F H K E U J · 것이 D N R F M · 많다 G H ⇧K A H ·
감사합니다 D H G R H V H B E M A H.

## 4. Shift, digits and symbols

No jamo sits on ⇧ of a number or symbol key, except ⇧¥ (ㄼ) and ⇧; (ㄲ). So digits and punctuation
are typed directly in henkan mode: nothing to predict, nothing to switch.

| Keys | Type |
|---|---|
| ⇧ + a letter key | the unshifted jamo, unless the key has a ⇧ final (J K L) |
| ⇧ + a number or symbol key | its own JIS character: 1–0 - ^ @ [ : ] , . / \ |
| ⇧⌥ + a number or symbol key | its shifted JIS character: ! " # $ % & ' ( ) = ~ \| ` { + * } < > ? _ |
| ⌥ + a key, without ⇧ | not bada's: what's staged is committed and the key goes to the app |
| not in henkan | ¥ and ; (⇧ on their keys is ㄼ and ㄲ): type them in muhenkan (英数) |

On a US ANSI keyboard the ¥ and ろ keys are missing; everything on them can still be typed: final ㅂ
with B, ㄼ with L B, ㅢ with , M.

## 5. Typo suggestions

What was typed always shows first. Readings one slip away are suggested under it, marked `typo?`
in the menu's note column; they never replace it, nor whatever ranks first among the enabled
layouts.

- **Language model:** a jamo 4-gram model (initial, vowel and final are distinct symbols) trained on
  `engine/data/ko-words.tsv`, each word weighted by its count, Witten–Bell interpolated:
  `engine/data/ko-jamo-4gram.bin` (149k n-grams, 1.4 MB), built by `engine/tools/build_jamo_lm.py`.
- **Slips** (one per reading; log10 cost):

  | Slip | Cost |
  |---|---|
  | D typed for the final ㅇ (J) | −0.7 |
  | the same consonant on its other key, e.g. K (final-only ㄴ) for E | −1.3 |
  | Shift missed or extra on a key whose ⇧ is another jamo | −2.0 |
  | **NKRO typing**: two neighbouring keys on different hands registered out of order | −0.8 |
  | NKRO typing: two neighbouring keys on one hand (different fingers) out of order | −1.3 |
  | NKRO typing: three keys of a roll reordered (all different fingers) | −1.8 |
  | the next key on the same row | −2.5 |

  NKRO typing is a fast roll whose keys register in the wrong order. Keys on one finger can't
  overlap, so they're never taken as reordered (fingers: touch typing on JIS, `typo.rs`).
  Example: ㅎ ㄴ ㅏ (V K H) suggests 한; 가맛합니다 (ㅏ and ㅅ rolled) suggests 감사합니다.

- **Shown** when its probability times its slip's is at least a tenth of the typed reading's (log10
  ratio ≥ −1.0), it isn't the same text, and it has fewer loose jamo than what was typed (or none).
  At most three; the user's strong negative words are left out, and so is a text another enabled
  layout already offers.
- **Measured** (`cargo run --release --example typo_eval`: 3,000 common words drawn by √count, one
  slip each, drawn by cost): the intended word is suggested for **98.5%** of slipped words (96.6%
  first); **5.2%** of correctly typed words get a suggestion.

## 6. F-key actions

Bound to nothing by default (spec §6.5).

| Action id | What it does to the staged input |
|---|---|
| `raw` | Every key as its jamo, nothing composed: `ㅎ ㅏ ㄴ` → ㅎㅏㄴ. Lasts until the commit |
| `prediction-1` … `prediction-5` | Commits the first … fifth menu entry (index 0 … 4) |
| `qwerty-fullwidth` | The staged keys as JIS QWERTY, full-width (ｖｈｋ). Lasts until the commit |
| `qwerty-halfwidth` | The same, half-width (vhk) |
| `hanja` | Hanja for the **staged text**; with nothing staged, for the **selection**; with no selection, a menu of **OECD countries' one-character hanja names** (美 미국, 英 영국, 日 일본 …) |

`prediction-N` does nothing when the menu has fewer entries, or while `raw` or a `qwerty-` action is on.

`hanja` opens a list (the hanja, with its meaning and reading where the dictionary has one: 學 배울 학)
for the whole text, else for the longest run of syllables at its end that has hanja (나는학교: 학교).
↓ Tab かな and ↑ ⇧Tab ⇧かな move in it; Enter or Space takes the highlighted hanja; Esc or ⌫ closes
the list and leaves the text as it was; any other key takes the hanja and then acts as usual. The
dictionary is `engine/data/hanja.tsv` (libhangul's, with Unihan readings:
`engine/data/LICENSE-libhangul-hanja.txt`).

The OECD list is `engine/data/oecd.tsv`. **[decide]** several members have no settled one-character
name in Korean usage (Estonia, Latvia, Lithuania, Slovakia, Slovenia, Costa Rica); the file lists
the longer transliterations for them, marked for review.

## Design principles

Its aims: every common jamo on an unshifted key, Korean typed without moving the wrist off the
home position, and digits and punctuation typed in the middle of Korean with no mode switch and no
prediction. 英数 and かな, under the thumbs either side of the space bar, change language.

What it takes from 2-set, 3-set and JIS kana, and what it leaves, is the table in §1. The goals of the
first rounds and the decisions that changed them are in the history (§H1, §H12), and why layout 13
went back to deterministic typing in §H17. In the owner's words: the [README](../../README.md#bada).

## History

How bada's layout got to what it is: the measurements, the rounds of candidates and the first
layout 13 (ranked, with 초성 input), in the order they happened. **Nothing here is current**: the
layout as built is §1–§6 above. Saved candidates are in `docs/history/`.

The sections keep the numbers they had when the design history was a document of its own, behind
an H: §H1–§H17 (it never had a seventh section, so there is no H7). A plain §N on this page is one
of §1–§6 above.

**Round 2 supersedes the key budget of round 1** (owner, 2026-11-07): Bada uses the **full JIS kana
space**, all 48 keys JIS Kana uses (1–0 − ^ ¥, Q–P @ [, A–L ; : ], Z–M , . / ろ). Shift on the 22
number and symbol keys is reserved for digits and symbols, as JIS Kana reserves it for its own
punctuation. Rounds 1's 26-key analysis (§H1–§H5) stays below for the record; §H8 is current.

### H1. Goals and constraints

1. **Typing efficiency and less hand fatigue** drive every choice. KTS / 2-set compatibility is
   not a goal.
2. ~~Only the 26 letter keys~~ (round 1). Round 2: the 48 kana-space keys, Shift on the 22
   non-letter keys reserved for digits and symbols (§H8).
3. **Efficient typing avoids Shift.** Shift is for rare things only.
4. Deterministic **[decide]**: one key sequence, one text. Henkan mode can rank several
   interpretations, but a layout that needs ranking to type plain Korean is harder to trust and to
   learn. Proposed: deterministic.

### H2. What the text costs (measured)

`tools/analysis/` reads 12M Hangul syllables, 3M each from news (Leipzig), formal writing
(petitions), reviews (NSMC) and conversation (korean-safe-conversation), from JIS Moonshot's
corpora. Shares are per syllable.

| | |
|---|---|
| initial ㅇ | **23.4%** of syllables: the most frequent initial by far (ㄱ is next, 12.1%) |
| …at word start / after an open syllable / after a final | 7.0% / 6.4% / 10.2% |
| a final consonant | 46.0% (ㄴ 14.8, ㄹ 8.1, ㅇ 7.6, ㄱ 4.5, ㅁ 3.0, ㅂ 2.4, ㅆ 2.1, ㅅ 1.2) |
| vowels | ㅏ 21.4, ㅣ 16.2, ㅡ 12.4, ㅓ 9.9, ㅗ 9.5, ㅜ 6.5, ㅕ 4.9, ㅐ 4.9, ㅔ 4.5, ㅘ 2.3, ㅛ 1.7, ㅢ 1.5, ㅚ 1.2, rest < 1 |
| 2-set | 2.53 keys per syllable; Shift on 2.0% of keys, half of it ㅆ (then ㄸ, ㄲ, ㅖ) |

Standalone jamo (ㅋㅋ, ㅎㅎ, ㅠㅠ) aren't counted yet; they matter in chat.

Takeaways:

- The **initial ㅇ** is the one big keystroke to win. It carries no sound; typing it is pure
  overhead.
- 2-set already puts Shift on only 2% of keys; the Shift that matters is **ㅆ** (했, 있, 었, 겠).
- ㅑ ㅠ ㅖ ㅒ are rare (under 1% each) and don't deserve keys of their own. ㅕ ㅐ ㅔ do.

### H3. Proposed rules

#### H3.1 Implied ㅇ

A vowel typed **at the start of a word, or right after a syllable without a final**, starts a new
syllable on ㅇ: `ㅏ` → 아, `ㄱ ㅏ ㅏ` → 가아, `ㅜ ㄹ ㅣ` → 우리.

- If the vowel **combines** with the vowel before it, it combines (`ㅂ ㅗ ㅏ` → 봐), and the two
  syllables need an explicit ㅇ (`ㅂ ㅗ ㅇ ㅏ` → 보아). That costs 0.5% of syllables (보이, 주어,
  수이…), against 13.4% of syllables saved.
- After a final the vowel takes the final, as in 2-set (`ㅁ ㅓ ㄱ ㅓ` → 머거); 먹어 is still
  `ㅁ ㅓ ㄱ ㅇ ㅓ`. Moving this case (10.2% of syllables) would need a syllable-boundary key or
  separate final keys, and 26 keys have no room for either.
- The ㅇ key still works everywhere; nobody has to use the implied ㅇ.

Measured: 2.53 → **2.39 keys per syllable (−5.3%)**.

#### H3.2 Vowels: keys and sequences

Keyed: ㅏ ㅓ ㅗ ㅜ ㅡ ㅣ ㅐ ㅔ ㅕ, plus two of ㅛ ㅘ ㅢ (§H4). The rest are typed as sequences the
automaton joins, the way 2-set already joins ㅗ+ㅏ:

| Vowel | Typed |
|---|---|
| ㅘ ㅙ ㅚ | ㅗ ㅏ, ㅗ ㅐ, ㅗ ㅣ |
| ㅝ ㅞ ㅟ | ㅜ ㅓ, ㅜ ㅔ, ㅜ ㅣ |
| ㅢ | ㅡ ㅣ |
| **ㅑ ㅠ** | **ㅣ ㅏ, ㅣ ㅜ** (new) |
| **ㅖ ㅒ** | **ㅕ ㅣ, ㅑ ㅣ** (new; off Shift) |
| ㅛ | ㅣ ㅗ (new), if ㅛ has no key |

ㅣ-first pairs never occur inside a Korean syllable, so they can't be misread; with the implied
ㅇ, 이아 / 이우 take an explicit ㅇ like 보아.

#### H3.3 Consonants

- 14 consonants keyed, with **ㅆ on its own key** (no Shift: the most frequent Shift in 2-set).
- **ㄲ ㄸ ㅃ ㅉ on Shift** of their plain consonant (0.5–0.9% of keys in total). Doubling a
  consonant can't type them deterministically: `ㄱ ㅏ ㄱ ㄱ ㅏ` is both 각가 and 가까.
- Double finals (ㄶ ㄺ ㅄ …) as in 2-set: two consonants in a row.

### H4. Candidate systems (measured)

Every system below was placed on the 26 keys by simulated annealing (`optimize.py`) under one
effort model: per-key cost (home row and index/middle fingers cheapest, pinky and bottom corners
dearest), same-finger bigrams penalised hardest, then same-hand row jumps, Shift at 2 extra. Effort
is per syllable; lower is better. The optimizer is free to mix consonants and vowels across hands;
it chose the 2-set split (consonants left, vowels right) every time.

| System | Keys/syl | Effort/syl | Shift | Same finger | Left hand |
|---|---|---|---|---|---|
| 2-set as it is | 2.527 | 3.351 | 2.02% | 2.86% | 58.2% |
| 2-set, keys re-placed | 2.527 | 3.105 | 2.02% | 1.89% | 58.2% |
| 2-set + implied ㅇ | 2.394 | 3.043 | 2.14% | 2.59% | 55.9% |
| **A** ㅆ ㅛ ㅘ keyed | 2.397 | 3.004 | 0.96% | 2.47% | 55.8% |
| **B** ㅆ ㅛ ㄲ keyed | 2.421 | 3.006 | **0.55%** | 2.47% | 55.1% |
| **C** ㅆ ㅛ ㅢ keyed | 2.406 | **2.987** | 0.95% | 2.03% | 55.7% |
| **D** ㅆ ㅑ ㅛ ㅠ keyed, ㅔ = ㅓ ㅣ | 2.445 | 3.024 | 0.94% | 1.94% | 54.5% |

Against 2-set: about **11% less effort, 5% fewer keys, half the Shift or less**. A, B and C are
within 1% of each other, which is inside the model's error: choose between them on feel.

#### H4.1 Draft board: C

```
 Q  W  E  R  T    Y  U  I  O  P
 ㅌ ㄷ ㄹ ㅈ ㅆ   ㅢ ㅓ ㅡ ㅜ ㅛ
  A  S  D  F  G    H  J  K  L
  ㅅ ㅇ ㄴ ㄱ ㅎ   ㅔ ㅏ ㅣ ㅗ
   Z  X  C  V  B    N  M
   ㅍ ㅋ ㅁ ㅂ ㅊ   ㅐ ㅕ
```

Shift: ⇧ㄷ ㄸ, ⇧ㅈ ㅉ, ⇧ㄱ ㄲ, ⇧ㅂ ㅃ (⇧ㅅ is ㅆ too). Everything else on Shift is free.

This board is a first optimizer output, not a design: the effort model is generic (no row
stagger, no JIS-specific reach, no hand-specific weights), and placement should be re-run once the
model is tuned and the open questions below are answered.

### H5. What 26 keys can't do

- **Left-hand load stays ~55%.** Consonants (initials and finals) outnumber vowels, and a final
  followed by the next initial is a same-hand pair. Fixing that needs finals on the vowel hand
  (3-set style), which needs about 7 more keys.
- **먹어-type syllables (10.2%) keep their ㅇ**, for the same reason.

Both are open to a bigger idea; within 26 deterministic keys, §H3 is about the ceiling.

### H6. Open questions

1. **[decide]** Deterministic, as proposed?
2. **[decide]** The implied ㅇ (§H3.1), with its "combining wins" rule?
3. **[decide]** The two spare vowel slots and the tense consonants: A (ㅛ ㅘ), B (ㅛ ㄲ, least
   Shift), C (ㅛ ㅢ, least effort), or another mix.
4. **[decide]** Keep the consonants-left / vowels-right split? (The optimizer prefers it.)
5. **[decide]** Shift's free slots: leave empty, or use for rare syllables/endings?
6. Which keyboard to tune the effort model on (Mac JIS, HHKB JIS, …): stagger and reach differ.
7. Standalone jamo in chat (ㅋㅋ, ㅠㅠ): measure and give them a cheap path.

### H8. Round 2: the 48 kana-space keys

#### H8.1 Shift on the number and symbol keys **[decide]**

Proposed: **⇧ + a number or symbol key types its unshifted JIS character** (⇧1 → 1, ⇧- → -,
⇧/ → /, ⇧ろ → \\), and **⇧⌥** types its shifted JIS character (⇧⌥1 → !, ⇧⌥/ → ?). Shift on the
26 letter keys stays free for Korean. Open: whether ⇧ should give the *shifted* character on some
keys instead (? and ! are frequent in chat; on `/` and `1` ⇧ could give ? and !).

#### H8.2 What 48 keys make possible

With 48 keys there is room for **separate initial and final keys** (3-set style), which round 1
couldn't fit:

- the syllable boundary is always known, so **every initial ㅇ can be implied**, after a final too
  (`ㅁ ㅓ ㄱ(final) ㅓ` → 먹어): 23.4% of syllables lose a key, not 13.4%;
- no jamo jumps between syllables (no 도깨비불): a syllable is final the moment the next initial
  or vowel arrives;
- ㄲ ㄸ ㅃ ㅆ ㅉ get keys of their own: no Shift for Korean at all, if wanted.

#### H8.3 Measured

Same corpora and method as §H4, now on 48 keys plus Shift on the letters (`optimize48.py`). The
number row and the outer right-pinky keys are dear in the effort model.

| System | Units | Keys/syl | Effort/syl | Shift | Same finger | Same hand | Left hand |
|---|---|---|---|---|---|---|---|
| 2-set as it is (§H4, 26 keys) | 26 | 2.527 | 3.351 | 2.02% | 2.86% | 18.6% | 58.2% |
| **2-set-48**: every vowel and tense consonant keyed, implied ㅇ | 40 | 2.325 | **2.870** | 0.11% | 1.41% | 21.9% | 49.9% |
| **3-set A**: 13 vowels, 16 finals | 48 | 2.267 | 2.988 | 1.30% | 0.80% | 22.2% | 47.4% |
| **3-set B**: 18 vowels, all 27 finals | 64 | **2.218** | 2.970 | 1.60% | 0.58% | 18.1% | 48.8% |
| **3-set C**: 15 vowels, 18 finals | 52 | 2.239 | 2.973 | 1.39% | **0.55%** | **16.9%** | 46.9% |

Reading it:

- Every 48-key system beats 2-set by 11–14% in effort and 8–12% in keys, and balances the hands
  (≈ 48% left against 58%).
- **3-set types 2–5% fewer keys and halves same-finger pairs**, but needs 48–64 keys against
  2-set-48's 40, so more of them land on the number row and the pinkies, and the generic model
  scores its effort ~4% higher. Which wins depends on how heavily the model weighs reach against
  keystrokes and same-finger pairs: this needs the owner's hands, not more annealing.
- The annealed 3-set boards mix initials and finals on both hands. That's hard to learn; a real
  3-set design would group them (initials on one hand, vowels and finals on the other, as 3-set
  390 and 3-final do). Next step: re-run with that constraint.

#### H8.4 Open questions (round 2)

1. **[decide]** 2-set-48 (fewer keys to learn, lowest modelled effort) or 3-set (fewest keys,
   implied ㅇ everywhere, no jamo jumps)?
2. **[decide]** §H8.1, Shift on the number and symbol keys.
3. **[decide]** For 3-set: group initials and finals by hand?
4. Tune the effort model on the HHKB JIS (the owner's daily board): especially how dear the number
   row and the keys right of P / L / ろ really are.

The drafts can be seen in the renderer (`tools/render`, §H9). (Then: the renderer has since been
cut down to the catalog's layouts, and no longer builds the drafts in.)

### H9. Reproducing

```sh
cd tools/analysis
python3 -m venv .venv && .venv/bin/pip install numpy
python3 jamo_stats.py            # §H2 frequencies
python3 ieung.py                 # §H3.1 where initial ㅇ appears
.venv/bin/python optimize.py     # §H4 systems and boards (26 keys)
.venv/bin/python optimize48.py   # §H8 systems and boards (48 keys) -> out_48.json
python3 ../render/build_layouts.py   # renderer data (then with the drafts; now the catalog's layouts only)
open ../render/index.html
```

`tools/render/index.html` draws any layout on the **HHKB JIS** (dimensions from the owner's HHKB
Keycap Swap artifact) or a **generic Apple-style JIS** board, with Shift legends, the JIS legends,
usage heat for drafts, and SVG export. Board geometry is in `keyboards.js`.

Set `BADA_CORPORA` if JIS Moonshot's corpora aren't at `~/Projects/jis-moonshot/corpora`.

### H10. Round 3 idea: token keys (2026-11-07, exploring)

The owner's idea: a key needn't be one jamo. It can be a **token** that stands for several (`F` =
ㅎ / ㅍ / ㅃ), and the henkan ranker picks the word, as kana-to-kanji conversion does.

Combined with 3-set roles (separate initial, vowel and final keys, every compound vowel and double
final a single key) this means **one key per jamo slot**: about 2.2 keys per syllable, never more,
and the syllable shape is always certain; only which member of a token is meant is guessed.

`tools/analysis/tokens.py` merges jamo greedily (within a role, the pair that adds the least
ambiguity), and `tokens_eval.py` re-scores the groupings on the top 100k words (85.5% of running
words). "Wrong" = a more frequent word has the same keys, by word frequency alone, no context:

| Keys | Wrong at top-1 | Not in top 3 |
|---|---|---|
| 62 (every jamo its own key) | 0 | 0 |
| 42 | 0.10% | 0.00% |
| 34 | 0.28% | 0.00% |
| **30** | **0.44%** | 0.00% |
| **26** | **0.88%** | 0.02% |
| 22 | 1.86% | 0.09% |

The 30-key grouping (greedy, not tuned for learnability):

- initials (13): ㄱ ㄴ ㄷ ㄹ ㅁ ㅂ ㅅ ㅇ ㅈ ㅎ, ㄸ/ㄲ/ㅍ, ㅉ/ㅊ/ㅋ, ㅌ/ㅃ/ㅆ
- vowels (10): ㅏ, ㅓ, ㅗ, ㅣ/ㅒ, ㅐ/ㅢ, ㅕ/ㅔ, ㅘ/ㅚ, ㅜ/ㅠ/ㅞ, ㅡ/ㅟ/ㅑ, ㅙ/ㅛ/ㅖ/ㅝ
- finals (7): ㄹ, ㄱ/ㄺ, ㄴ/ㄲ/ㄼ/ㄵ, ㅁ/ㄷ/ㅀ, ㅂ/ㅌ, ㅅ/ㅊ/ㅄ/ㅍ/ㅈ/ㄻ, ㅆ/ㅇ/ㅎ/ㄶ

With the implied ㅇ the initial ㅇ key goes too: 29 keys, inside the letter area with room to
spare, so **every symbol key could keep its JIS character** again.

Caveats, not yet measured:

- **In-sample and unigram.** The vocabulary that scores the groupings also chose them, and the
  real ranker has context (better) but meets names, slang and new words outside the top 100k
  (14.5% of running words; worse). Next: held-out text, eojeol context, and the OOV share.
- **Learnability.** Greedy groups are arbitrary (ㅌ/ㅃ/ㅆ). Groups constrained to be mnemonic
  (sound families ㄱ/ㅋ/ㄲ, shapes) will cost some ambiguity; how much is the next measurement.
- **Escape hatch.** A deterministic way to force a member (e.g. Shift+token = its second member,
  or cycling inside a syllable) for names and ㅋㅋ.
- **Feel.** The preedit can change as you type, as in Japanese conversion; at 0.5–0.9% wrong that
  is one correction every ~110–230 words.

### H11. Round 3 idea: chords (2026-11-07, exploring)

The owner: simultaneous presses aren't claimed by any global setting, so Bada can take chords.

Korean suits chords better than most scripts: a syllable is already a fixed set of roles
(initial, vowel, final). With role-separated keys (§H10) **one chord can type one syllable**, as
Korean stenotype machines do. And because the roles use different keys, the order of the keys
inside a syllable carries no information: a chord and a fast roll type the same thing.

Questions to settle before designing:

- **Hardware rollover.** A chord of 3–4 keys needs NKRO or at least ghost-free 4-key combinations on
  the keys used. To check: HHKB Hybrid Type-S over USB and Bluetooth, and Apple JIS boards.
- **InputMethodKit key-up.** Release-based strokes (a stroke ends when every key is up) need key-up
  events in the input controller (`recognizedEvents` with key-up). To verify per app in the first
  macOS spike; without it, chords have to be timing-based on key-down only.
- **Stroke boundaries under rollover.** Fast sequential typists overlap keys across syllables.
  Proposed rule: parse in press order by role (initial → vowel → final starts a new syllable
  whenever the role goes back), so a chord, a roll and plain sequential typing agree.
- **What chords are for.** (a) only speed: any order inside a syllable; (b) more symbols from fewer
  keys: e.g. vowels as chords of 3–4 keys, so fewer keys and fewer tokens; (c) whole-word or ending
  briefs (했습니다, 있는) as steno does.

### H12. Decisions (2026-11-07, the owner)

- **Ranked is fine.** Bada may be non-deterministic; n-grams trained on the corpora compensate for
  token ambiguity (§H10). This supersedes "deterministic" in §H1.
- **Home keys cost nothing.** A S D F and J K L ; carry **zero** effort in the model: the fingers
  rest there. (The model so far charged them 0.7–1.3.)
- **One token, several inputs.** A token may be reachable by more than one key or chord, and should
  be where it helps ergonomics (e.g. a frequent token on a home key and in a chord).
- **Chord limits** (after the NKRO check): normal typing chords **at most 2 keys**; very fast
  typing **at most 3**. Larger chords are out.
- **Chords are only an accelerator.** Every letter must also be typeable **one key at a time**
  (1KRO): **a chord is never required**. Navigating predictions with the arrow keys counts as
  1KRO too, so picking a non-top reading needs no chord either. So the sequential layer
  is complete on its own (§H10's token keys), and chords add speed on top (§H11 (a)–(c), within the
  2/3-key limits).

### H13. Token keys placed (effort model v2, 2026-11-07)

Model v2: A S D F J K L ; cost **zero**; Shift unused (1KRO); initial ㅇ always implied (a vowel
key after a vowel or final always starts a syllable, since every vowel token is one key).
`place_tokens.py` places §H10's greedy groupings on the 48 keys. Standard 2-set re-scored under v2
for comparison.

| | Keys | Keys/syl | Effort/syl | Wrong top-1 (unigram) | Same finger | Pinky |
|---|---|---|---|---|---|---|
| 2-set as it is | 26 + Shift | 2.527 | 2.213 | 0 | 2.86% | 8.3% |
| tokens, step 32 | 29 | **2.215** | 1.869 (−16%) | 0.44% | 0.48% | 18.9% |
| tokens, step 36 | **25** | **2.215** | **1.737 (−22%)** | 0.88% | 0.44% | 18.3% |
| tokens, step 40 | 21 | 2.215 | 1.610 (−27%) | 1.86% | 0.90% | 14.9% |

Seen in the renderer (layouts 312–314, no longer built into it):

- The optimizer put **vowels on the left hand and consonants on the right**, initials toward the
  centre and finals on the outer right (L ; : , . /). The number row went unused at 25 keys.
- At 25 keys, Q T Z X C B are empty: room for second inputs of frequent tokens (§H12) or chords.
- **Pinky load doubled** (8% → 18%): with A and ; free, the model loads them. **[decide]** keep
  resting keys at zero for every finger, or give the pinky home keys a small cost.
- The groups are still greedy and unlearnable (§H10); the next round constrains them.

### H14. Candidates

Hand-built in the layout editor (`tools/editor`) and saved under `docs/history/`. Scored with
`tools/analysis/evaluate_layout.py` (model v2; duplicates taken by the cheapest route through each
word; ambiguity not scored).

| Candidate | Keys/syl | Effort/syl | Shift | Same finger | Same hand | Pinky | Left |
|---|---|---|---|---|---|---|---|
| 2-set (KTS), reference | 2.527 | 2.213 | 2.02% | 2.86% | 18.6% | 8.3% | 58.2% |
| 1, initial ㅇ explicit (as first saved) | 2.457 | 2.528 | 3.49% | 2.57% | 15.3% | 19.3% | 56.5% |
| 1 `docs/history/bada-candidate-1.json`, **implied ㅇ** (owner, 2026-11-07) | **2.317** | 2.570 | 3.70% | 2.96% | 19.5% | 20.5% | 54.1% |
| **2** `docs/history/bada-candidate-2.json` "초성입력가능" (owner, 2026-11-07; **preferred**) | 2.323 | 3.414 | 0 | 2.44% | 46.6% | 25.3% | 37.9% |

Candidate 1: 2-set-style consonants (초·종성 on one key) on the left, vowels on the right home row
with ㅕ ㅑ ㅛ ㅠ on Shift, every compound vowel and tense consonant keyed (number row), double
finals on the bottom right, and right-hand duplicates of ㄱ ㄴ ㄹ ㅁ ㅇ and left-hand duplicates
of ㅏ ㅣ. Implied ㅇ (2026-11-07; first saved with it off): a vowel at word start or after an
open syllable starts a syllable on ㅇ. Every compound vowel has a key, so **[decide]** whether
ㅗ+ㅏ still joins into ㅘ (then 보아 needs an explicit ㅇ) or never joins (then `ㅂ ㅗ ㅏ` is 보아).
The implied ㅇ saves 5.7% of keys but scores slightly *worse* in effort: ㅇ sat on F, a resting key
the model charges nothing, and dropping it puts more vowel → vowel pairs on the right hand. It puts jamo on Shift of four symbol keys (0 ⇧ㅙ, ] ⇧ㄱ, / ⇧ㅀ,
ろ ⇧ㄿ), which §H8.1 reserves for symbols.

Candidate 2 ("초성입력가능", the owner's preferred): every consonant on its own unshifted key in
both roles (2-set style), across the home and Q rows of both hands (S D F G | H J K L hold ㅅ ㅈ
ㅇ ㄹ | ㄴ ㄱ ㄷ ㅎ); vowels on the edges (A ㅗ, Z ㅜ, Q ㅕ, : ㅓ, , ㅏ, . ㅣ, / ㅡ, [ ㅔ, ] ㅐ) and
the number row (compound vowels, ㅑ ㅠ ㅛ, second ㅏ on 1 and 3); duplicates for ㅅ ㅆ ㅇ ㄷ ㅃ;
no Shift; implied ㅇ. Double finals other than ㅄ are typed as two consonants.

Its model scores come from the vowels: ㅏ (21% of syllables) sits on `,` 1 3, ㅣ (16%) on `.`, ㅡ
(12%) on `/` and ¥, ㅗ on A, ㅜ on Z, so the pinkies carry 25% and consonant → vowel pairs land on
the same hand (46.6% same-hand, against 2-set's 18.6%). 초성 input doesn't depend on that
placement (§H15): any layout with every initial on an unshifted consonant key supports it.

### H15. 초성 input (owner, 2026-11-07; superseded by §H17)

Bada accepts **초성-only typing** as an abbreviation, ranked like any other reading:

- **Only consonants typed** in the segment: the keys are read as a 초성 string and the menu offers
  the words that start with those 초성 (ㅇㄴㅎㅅㅇ → 안녕하세요), ranked by the n-gram model. The
  literal jamo string is always an entry too (ㅋㅋㅋ, ㅎㅎ, ㅇㅇ, ㅅㄱ are common on their own).
- **At least one 중성** in the segment: normal composition, as in any Korean method; the 초성
  reading is dropped.
- **Requirement on the layout:** every initial consonant on an unshifted key, not shared with a
  vowel. Candidate 2 meets it; so do 2-set and candidate 1.

How much a 초성 string alone says (word frequency only, no context; the n-gram ranker does better):

| Word length | Share of words | Right at top-1 | In top 5 |
|---|---|---|---|
| 1 syllable | 9.3% | 39.7% | 75.1% |
| 2 | 25.9% | 31.6% | 60.0% |
| 3 | 32.3% | 33.5% | 60.5% |
| 4 | 18.4% | 47.1% | 78.6% |
| 5 | 9.2% | 70.3% | 96.0% |

So 초성 input is a shortcut for long and frequent words and set phrases, with the menu in play;
everyday short words are faster typed in full.

#### H15.1 ⇧ + consonant = consonant + ㅏ (owner, 2026-11-07)

**Shift on a consonant key types that consonant with 중성 ㅏ**: ⇧ㅎ is a ㅎ+ㅏ token.

- **In 초성 input it does not end 초성 input.** The token is still a 초성 token, now known to carry
  ㅏ: ㅇㄴ⇧ㅎㅅㅇ reads as ㅇ ㄴ 하 ㅅ ㅇ and still predicts 안녕하세요. Only a vowel key starts
  normal composition. The final stays **open** (owner, 2026-11-07): ⇧ㅎ matches 하, 한, 할, 합 …
- **A consonant after a ⇧ token can be its final** (fix, 2026-11-07): the shown reading is ranked
  with the 초성 predictions. The normal composition (⇧ㄱ ㄴ → 간) wins when it's a known word, and
  ranks high when it's whole syllables; when it leaves loose jamo (ㅇㄴ⇧ㅎㅅㅇ composes ㅇㄴ핫ㅇ),
  the 초성 reading (안녕하세요) shows and the composition is listed.
- **In normal composition** it types the consonant and ㅏ as if both keys were pressed (⇧ㄱ → 가,
  ⇧ㄱ ㄴ → 간), as JIS Moonshot's Shift+consonant does (moonshot.md §9.2).
- Shift on the number and symbol keys stays reserved for digits and symbols (§H8.1): a consonant
  that is only on a symbol key has no ⇧ token there. In candidate 2, ㅃ is on R and `;`, so ⇧R is 빠.
- ㅏ is the 중성 of 21% of syllables, so the hint is free often. Marking every ㅏ syllable sharpens
  the 초성 reading a lot (word frequency only):

  | Word length | top-1 plain → with ⇧ㅏ | top-5 plain → with ⇧ㅏ |
  |---|---|---|
  | 2 | 31.6% → 43.7% | 60.0% → 73.0% |
  | 3 | 33.5% → 48.9% | 60.5% → 76.3% |
  | 4 | 47.1% → 65.3% | 78.6% → 91.9% |
  | 5 | 70.3% → 83.9% | 96.0% → 99.2% |

- **[decide]** 초성 input for the other Korean methods (10–12) too, or Bada only.
- **[decide]** the segment: a word up to a space, or the whole preedit (several words: ㄱㅅㅎㄴㄷ →
  감사합니다 spans one word; ㅇㄴㅎㅅㅇ ㅂㄱㅅㄴㄷ spans two).

### H16. Layout 13 was candidate 2 (owner, 2026-11-07; superseded by §H17)

JIS Bada (id 13) is `docs/history/bada-candidate-2.json` with §H15 (초성 input) and §H15.1 (⇧ + consonant
= consonant + ㅏ). Rules the build adds where the candidate is silent:

- **Compound vowels never join.** Every compound vowel has a key, so a vowel after a vowel always
  starts a new syllable on the implied ㅇ: `ㅂ ㅗ ㅏ` is 보아; ㅘ is the 0 key. **[decide]**: the
  alternative (ㅗ + ㅏ still joins) was offered and not yet chosen.
- **A final moves as its key's initial.** A consonant key typed as a final that a vowel then pulls
  into the next syllable comes back as *that key's* 초성: R typed after 가 is ㅄ; R then ㅏ is 가빠,
  not 갑사. A double final built from two keys gives up its second key.
- **Double finals other than ㅄ** are two consonant keys (ㄴ ㅎ → ㄶ), as in 2-set.
- **The segment** is the whole preedit: Space commits (spec §6.3), so a 초성 string is one word or
  phrase typed without spaces.
- **One consonant alone** shows as itself; predictions start at two 초성 tokens (one consonant is
  how every word starts, and a guessed word there would flash on each).

- **1 is ㅓ** (owner, 2026-11-07), the 중성 alone (with the implied ㅇ: 어); it was a second ㅏ.
  ㅏ stays on 3 and `,`; ㅓ is on `:` too.

#### H16.1 F-key actions (owner, 2026-11-07)

Bada publishes these actions (spec §6.5). **No F key is bound to any of them by default.**

| Action id | What it does to the staged input |
|---|---|
| `raw` | Strips all processing: no implied ㅇ, no implied ㅏ, no 종성, no predictions. Every consonant key is its 초성 (R is ㅃ, not ㅄ), every vowel key its vowel, nothing composed: `ㅎ ㅏ ㄴ` → ㅎㅏㄴ. Lasts until the commit |
| `prediction-1` … `prediction-5` | Uses the first … fifth ranked entry (menu index 0 … 4): commits it |
| `qwerty-fullwidth` | The staged keys as JIS QWERTY, full-width (ｈａｎ). Lasts until the commit |
| `qwerty-halfwidth` | The same, half-width (han) |
| `hanja` | Hanja for the **selection**; else for the **staged text**; else a menu of **OECD countries' one-character hanja names** (美 미국, 英 영국, 日 일본 …) |

(As built since: the staged text first, then the selection: §6.)

The OECD list is in `engine/data/oecd.tsv`. **[decide]** several members have no settled
one-character name in Korean usage (Estonia, Latvia, Lithuania, Slovakia, Slovenia, Costa Rica);
the file lists the longer transliterations for them, marked for review.

### H17. Rev2: layout 13 is deterministic (owner, 2026-11-08)

Candidate 2 typed too little to be read reliably: 초성 strings, ⇧ consonants as + ㅏ and the implied
ㅇ all had to be guessed, and the guess was often wrong. The owner went back to **deterministic
typing** (the same keys always give the same text, as 2-set does) while keeping what candidate 2 did
well: Korean on the 48 kana-space keys with no Shift for any common jamo.

Layout 13 is now `docs/layouts/13-bada.json` ("bada rev2 v2"; an earlier draft, "draft rev2 v1", is
saved as `docs/history/rev2-draft-v1.json`). Its rules:

- **Initials, finals and vowels.** Consonants on the left half and the left of the number row are
  initial-or-final keys: a final, until a vowel takes it as its initial (2-set). Four keys on the home
  row's right (J K L ;) and ¥ are **final-only**: ㅇ ㄴ ㄹ ㄱ ㅂ, the five commonest finals; a vowel
  never takes them, so what they type never moves. F (ㅇ), 3 (ㅉ), 5 (ㄸ), Q and \ (ㅃ, final ㅄ) are
  initial-only or keep their own final. (Later the same day Q became initial-only ㅃ, and ㅄ is typed on
  `]` alone: §2.)
- **No implied ㅇ**: ㅇ is typed (F). A vowel with nothing before it is the bare vowel.
- **Every vowel has a key**, compound ones too (number row, Y, [, ], ろ); two vowel keys still join as
  in 2-set (ㅗ ㅏ → ㅘ).
- **Double finals**: two finals in a row join (ㄹ ㅁ → ㄻ), or one ⇧ key: ⇧J ㅁ, ⇧K ㄶ, ⇧L ㄺ, ⇧; ㄲ,
  ⇧¥ ㄼ. ⇧. and ⇧/ are punctuation (. and /; ㅛ ㅠ are on 9 and 8).
- **Shift elsewhere**: on a letter key, the unshifted jamo; on a number or symbol key, its JIS
  character (§H8.1); ⇧⌥ its shifted JIS character. Jamo stay off ⇧ of the number and symbol keys
  (but ⇧¥ ㄼ, ⇧; ㄲ) so that digits and punctuation never need a prediction or a mode switch; 英数
  and かな sit under the thumbs either side of the space bar for changing language. Only ¥ and ;
  can't be typed in henkan. The comparison with 2-set, 3-set and JIS kana is the table in §1.
- **No 초성 input.** Consonants alone are those consonants. The 초성 machinery (prediction, shortcuts,
  `choseong` diagnostics) stayed in the engine unused at first and has since been taken out;
  **[decide]** whether to bring it back on request (a key that turns typed consonants into a word
  menu).
- F-key actions as §H16.1 (`raw` is now simply every key as its jamo).

Typo suggestions, added the same day, are part of the current layout: §5.

## Open questions

1. **[decide]** ⌥¥ and ⌥; (no Shift) typing ¥ and ;, so that every JIS character can be typed in henkan.
2. **[decide]** learning the slips' costs (§5) from real typing (the learner-app logs backspaced keys).

## Tools

- **Key map:** `13-bada.json`, the reference the engine and the renderer follow. The Mac build also
  writes the Keyboard Viewer's henkan legends from it (`bada.keylayout`, by
  `mac/tools/make-keylayout.swift`; spec §4.1).
- **Board images and the interactive renderer:** `tools/render` (`python3 tools/render/build_layouts.py`
  after changing `13-bada.json`, then `export_png.py`).
- **Key drill:** `tools/practice/index.html`. **Your own typing, logged and exported:**
  `utils/learner-app`.
- **Typo suggestions:** the model is built by `engine/tools/build_jamo_lm.py`, and measured by
  `cargo run --release --example typo_eval` in `engine/` (§5).
- **The history's measurements:** `tools/analysis`, as §H9 runs them.

Images: `python3 tools/render/export_png.py` (HHKB JIS). Interactive: open `tools/render/index.html`.
