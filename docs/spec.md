# bada ime: spec

Status: **draft** (2026-11-07). Items marked **[decide]** are not settled: don't implement a guess
silently. Where the build differs from the design, or has settled an open item for now, the text says
"as built" (the build is 0.3.1; §7.0 sums it up). The Bada layout itself (id 13) is designed in
[layouts/13-bada.md](layouts/13-bada.md); every layout has its page in [layouts/](layouts/README.md).

## 1. Names

| | Project | Full | Short |
|---|---|---|---|
| id | `bada-ime` | | |
| Korean | | 바다 입력기 | 바다 |
| Japanese | | バダIME | バダ (always katakana) |
| English | | bada ime | bada (always lowercase) |

Repo: `team-mirae/bada-ime`, open source, MIT, copyright Mirae Research.

Most of what bada ime needs already exists in **JIS Moonshot** (`team-mirae/jis-moonshot`) and
**JIS Korean** (`team-mirae/jis-korean`): the engine, the Hangul automaton, the kana layout and
automaton, the predictor and its corpora tools, and the macOS InputMethodKit shell. bada ime
reorganises them around two modes (muhenkan, henkan) and a pool of layouts.

## 2. Scope

- **macOS only** as an input method. No Windows. The web simulator (`/web`, §7) is the same engine
  behind a text field in a page, for trying bada ime out: not an input method.
- **JIS keyboards only** (日本語配列). Keys are physical positions with JIS legends; on any other
  keyboard layout the symbol keys type the wrong symbols. No ANSI/ISO modes.
- **Caps Lock is bada ime's** (owner, 2026-11-11; §6.7), once Accessibility and Input Monitoring
  are allowed; without them it's macOS's.

## 3. Vocabulary

Canonically **무변환 모드** / **변환 모드**; in the project, **muhenkan mode** and **henkan mode**
(Japanese readings).

| Term | Meaning |
|---|---|
| layout | A mapping from keystrokes to text, with an integer **layout id** (§5) |
| muhenkan layout | A layout that types a letter or sends a keycode per key. It **cannot compose**: no preedit |
| henkan layout | A layout that composes, as CJK input methods do: keys build a preedit, which is committed |
| deterministic | A henkan layout whose keys have exactly one reading (the composing Korean layouts, 10–14) |
| ranked | A henkan layout that keeps several readings of the same keys and ranks them by word counts (Korean typed as it sounds: 15, 18, 19) |
| non-deterministic | A henkan layout with several readings for the same keys (Japanese: kana → kanji) |
| search | A henkan layout whose keys spell a search, and whose menu lists what it finds (150, 151) |
| candidate | A layout enabled in henkan mode. Muhenkan layouts can be candidates too |
| interpretation | One reading of the staged keys, by one candidate (or one of its conversions) |

## 4. Modes and input sources

### 4.1 Two input sources, no more

bada ime registers exactly two macOS input modes, and never more:

| Input source | Reports as | Role |
|---|---|---|
| **Bada Muhenkan** | English (`en`, Roman) | muhenkan mode |
| **Bada Henkan** | Japanese (`ja`) | henkan mode |

macOS routes 英数 to the current English/Roman source and かな to the Japanese one. Layout
switching inside a mode is bada ime's own state, not a macOS input source. System Settings lists the
two as **bada (muhenkan)** and **bada (henkan)** (バダ（無変換）, バダ（変換）; 바다 (무변환), 바다 (변환));
their ids are `me.sjun.inputmethod.Bada.Muhenkan` and `….Henkan`.

**Mode changes are macOS's own switch.** The key tap (§6.1) catches 英数, かな and the profile keys,
but never selects an input source itself: a key that changes mode (英数 from henkan, かな or a
profile key from muhenkan, HYPER/MEH+かな) is reposted as a plain 英数 or かな, and macOS switches.
The input method then acts on the key when its mode is selected (`activateServer` / the input-mode
`setValue`): e.g. which henkan profile the key asked for. If macOS hasn't switched within 0.3 s, bada
ime selects the mode itself. Keys that don't change mode (かな in henkan, a profile key in henkan)
act in the input method directly.

**Setup after install** (from JIS Moonshot's `mac/README.md`): add **ABC – Extended**, then remove
**ABC**, and remove every other Japanese input method. Then 英数 always lands in Bada Muhenkan and
かな in Bada Henkan. (ABC – Extended stays installed because macOS keeps one plain keyboard; it
still types in password fields.)

**The Keyboard Viewer** shows the keyboard layout the input method sets (`overrideKeyboard`): in
muhenkan, ABC (with ろ as `\`); in henkan, **bada plain's (16) legends**, whatever layouts the profile uses
(one board can't show them all). That layout is `bada.keylayout`, hidden from the input source list and shipped in the
app (`mac/tools/make-keylayout.swift`): ABC as a JIS keyboard has it, with bada plain's legends on the
unshifted and ⇧ layers (its jamo and punctuation; with ⇧ the syllables and digits), and on a Caps
Lock layer of its own, which shows the unshifted layer (henkan
ignores Caps Lock, §6.7). Its ⌥, ⌘ and ⌃ layers are ABC's, so shortcuts are unchanged; a key the
engine passes on that carries a legend there is typed as ABC types it, by the input method. Muhenkan's
`qwerty.keylayout` is ABC with ろ as `\` (⇧ `_`), and has the same Caps Lock layer: ABC's capitals.
Without that layer a keylayout's Caps Lock types its unshifted layer, lowercase. Both files are
written at build time from ABC and `docs/layouts/16-bada-plain.json`.

### 4.2 Muhenkan mode

QWERTY passthrough: practically the same as JIS English in JIS Moonshot. Only muhenkan layouts run
here; the active one is the **current muhenkan layout** (today only id 0 exists). Nothing composes,
nothing is staged (but the letter a ⌥ + letter variants list shows, §6.2).

### 4.3 Henkan mode

- **No sub-modes.** Every enabled candidate reads every keystroke at the same time, and the
  interpretations are **ranked as you type**; the top one shows in the preedit.
- With one candidate enabled there is no ambiguity, unless that candidate is non-deterministic
  (Japanese).
- Muhenkan layouts can be candidates. Enabling **0, 12, 20** reproduces today's JIS Moonshot
  (QWERTY + Moonshot Korean + JIS kana, predicted).
- The ranker is JIS Moonshot's predictor, generalised from its fixed three readings to the set of
  enabled candidates: a **key model** per layout says how much the staged keys look like its
  language typed on it, and the layouts of a set are compared by that (§7.0). **[decide]** how a
  candidate without a model is scored. As built: 0, 10–16, 18, 20 and 21 have a model; 19 and the
  searches (150, 151) have none and keep their rule scores (§7.0).
- A layout reads the staged keys only if it has a meaning for every one of them; one that doesn't
  offers nothing until the keys are committed.

## 5. Layouts

### 5.1 Id ranges

| Range | Use |
|---|---|
| 0–9 | Latin |
| 10–19 | Korean |
| 20–29 | Japanese |
| 30–49 | Sino |
| 50–99 | other languages |
| 100–149 | symbols, emoji, braille, IPA, other |
| 150–199 | reserved by Mirae Research |
| 200–`UINT32_MAX` | free to use |

Ids are `u32`. **Development / debug** ranges (re-numbered when merged to main): 300–399 Korean,
400–499 Japanese, 500–599 Sino, 600–999 other.

### 5.2 Catalog

Every layout's page in [layouts/](layouts/README.md) opens with its metadata (names in English,
Korean, Japanese and its native script, short names, a unique in-code name, the reliability it claims,
abbreviations, designer, maintainer, contact) and is governed by the maintainer rules there. A page is
self-contained: the layout's keys and rules, its design principles and its history are on it; this
spec has what the layouts share.

| Id | Name | Display name | Kind | From |
|---|---|---|---|---|
| 0 | JIS QWERTY | `QWERTY` | muhenkan | JIS Korean / JIS Moonshot `A_QWERTY` (JIS English) |
| 10 | JIS KTS | `Korean (2-set)` | henkan, deterministic | standard KS X 5002 2-set on the JIS letter keys, JIS symbols untouched |
| 11 | JIS Korean Legacy | `Korean (legacy-korean)` | henkan, deterministic | JIS Korean Mode A |
| 12 | JIS Moonshot | `Korean (legacy-moonshot)` | henkan, deterministic | JIS Moonshot Mode A's Korean reading (moonshot.md §9) |
| 13 | JIS Bada | `Korean (bada)` | henkan, deterministic | new: [layouts/13-bada.md](layouts/13-bada.md) |
| 14 | JIS 3-set (세벌식 JIS, K3S-J) | `Korean (3-set JIS)` | henkan, deterministic | new: [layouts/14-3set-jis.md](layouts/14-3set-jis.md), designed by the owner from 세벌식 순아래; 모아쓰기 (jamo of a syllable in any order) |
| 15 | bada roman | `Korean (bada roman)` | henkan, ranked (the default reading, or a better-known one, shows; the English dictionary never first) | new: [layouts/15-bada-roman.md](layouts/15-bada-roman.md) |
| 16 | bada plain | `Korean (bada plain)` | henkan, deterministic | new: [layouts/16-bada-plain.md](layouts/16-bada-plain.md), a candidate to replace 13 (owner, 2026-11-12); the default henkan candidate |
| 18 | JIS Korean Roman | `Korean (Roman)` | henkan, ranked (the default reading shows; other readings offered) | new: [layouts/18-korean-roman.md](layouts/18-korean-roman.md) |
| 19 | JIS Kana Korean | `Korean (Kana)` | henkan, ranked (the table's reading shows; other pronunciations offered) | new, minor: [layouts/19-kana-korean.md](layouts/19-kana-korean.md) |
| 20 | JIS Kana | `Japanese (Kana)` | henkan, non-deterministic (kanji conversion) | JIS Moonshot `A_KANA`: [layouts/20-kana.md](layouts/20-kana.md), §11 |
| 21 | JIS Romaji | `Japanese (Romaji)` | henkan, non-deterministic (kanji conversion) | new: [layouts/21-romaji.md](layouts/21-romaji.md), §11 |
| 150 | Symbols | `Symbols` | henkan, search | Mirae Research: [layouts/150-symbols.md](layouts/150-symbols.md) |
| 151 | Unicode | `Unicode` | henkan, search | Mirae Research: [layouts/151-unicode.md](layouts/151-unicode.md) |

Every layout declares whether it is muhenkan or henkan.

### 5.3 Settings

- the **enabled set** of henkan candidates (default: bada plain, 16); earlier in the list wins a tie (the
  settings window keeps the list in catalog order);
- the **default muhenkan layout** (英数 enters it);
- the **default henkan target** for HYPER+かな (§6.1), and the **cycle order** for MEH+かな.
  Without a user order: integer id ascending. (Not built: §7.0.)
- a **weight** per layout (0–10, default 1): a multiplier on the score of every entry that layout
  raises, in every profile (before the user's negative words and the typo suggestions; in a set,
  before the key models' evidence: §7.0);
- **henkan profiles** (§6.6), off by default: candidates per profile key, links, recorded keys, what
  かな does in each profile, the cycle order, F keys per profile;
- **script forms** (§6.4), off by default, and the method that offers them;
- the **F keys** (§6.5): intercepted or not (default: intercepted), and each key's binding
  (default: none);
- the **emoji skin-tone order** for layout 150;
- layout 15's **Japanese romaji readings as shortcuts**, on by default
  ([layouts/15-bada-roman.md](layouts/15-bada-roman.md));
- **Caps Lock** (§6.7): toggle (default), transient or respect, and its muhenkan rule: ALL CAPS
  (default) or UPPER_SNAKE (§6.2).

The settings window (the input menu's "bada ime Settings…") has them on its Layouts, Profiles, Menu
and F Keys tabs, with Dictionary (§8), Backup and About. Backup → Export Settings… writes all of them
to a file (`{"format": "bada-settings/1", "settings": {…}}`); Import Settings… replaces them all from
one (fields a file lacks take their defaults). The Mac keeps them in its user defaults; the web
simulator in the browser's local storage, and the same file moves between the two.

## 6. Keys

**HYPER** = ⌘⌃⌥⇧. **MEH** = ⌃⌥⇧.

### 6.1 Everywhere

| Key | Action |
|---|---|
| 英数 | Enter muhenkan mode, on the **default** muhenkan layout. From henkan mode, commit what shows first |
| かな | Enter henkan mode. Already in henkan mode: see §6.3 |
| HYPER+かな | **In henkan:** back to the most recently used muhenkan layout. **In muhenkan:** back to the most recently used henkan; else the user's default; else the lowest enabled id. A muhenkan layout chosen as a candidate inside henkan mode **does not count** as use of that muhenkan layout |
| MEH+かな | Cycle to the next enabled input source, in the user's order, else id ascending |

**[decide]** "most recently used henkan" and MEH+かな's "input source" imply more than one henkan
target, while §4.3 says henkan mode has no sub-modes. Which of these is meant?

- (a) a **henkan target** is either "all enabled candidates" or one candidate locked alone, and
  MEH+かな cycles through muhenkan layouts and those targets;
- (b) there are several named **henkan profiles** (each an enabled set), and the hotkeys pick a
  profile;
- (c) only one henkan exists, so HYPER+かな from muhenkan always just enters henkan, and MEH+かな
  cycles muhenkan layouts only.

As built: HYPER+かな and MEH+かな both switch to the other mode (§7.0), and so does Caps Lock on its
default setting (§6.7). 英数 and かな are caught system-wide by a key tap once Accessibility is
allowed (the input menu's "Allow 英数 and かな Everywhere…", shown until it is): without the tap they
act only when macOS passes them to the input method, and it doesn't always pass かな on in henkan
mode. The same menu item asks for Input Monitoring (§6.6, §6.7).

### 6.2 In muhenkan mode

| Key | Action |
|---|---|
| ⌥Space | `_` |
| Caps Lock on | The **muhenkan rule** (§6.7): **ALL CAPS** (default), letters in capitals, as Caps Lock always types; or **UPPER_SNAKE**, capitals with Space typing `_` and ⌥Space a space. While bada ime handles Caps Lock, the input method types the capitals itself (§6.7) |
| ろ (`_` key, IntlRo) | `\`; ⇧ `_` (as on every JIS keyboard; macOS's ABC layout would give `_` for both). The engine types it, in muhenkan and wherever henkan passes the key on |
| ⌥ + letter | its **variants**, as JIS English did (jis-korean spec §3.6.3): the letter shows with a row of accented variants (⌥e: è é ê ë ē ė ę; ⇧ for capitals), none chosen. 1–9, 0 pick; the same ⌥ + letter again or the arrows choose; Enter commits; Esc / ⌫ cancel; any other key keeps what shows and goes on. A letter without variants types macOS's ⌥ character. In terminals ⌥ goes to the app untouched |
| 半角/全角 | **full-width** applied to the selection |
| ⇧半角/全角 | **half-width** applied to the selection |

半角/全角 is the `Backquote` position (left of 1 on a full JIS board, at the bottom left on the HHKB
JIS; Apple JIS boards don't have it). The conversion rules are JIS Moonshot's (moonshot.md §4.1). **[decide]** with no selection:
JIS Moonshot falls back to the word under the caret; keep that, or do nothing? As built: nothing.

### 6.3 In henkan mode

| Key | Action |
|---|---|
| ⇧Enter | Swallow Shift and Enter, **commit the preedit verbatim**, and do nothing else: never a newline, even where ⇧Enter normally makes one |
| Space, Enter, Esc, Tab, … | Commit the preedit as shown, **then** perform the key's normal action |
| かな, Tab, ↓ | Next entry in the suggestion menu (§6.4) |
| ⇧かな, ⇧Tab, ↑ | Previous entry; from entry 0 into the negative entries (§6.4) |
| ⌃かな | **Fixes what shows** (§6.4): the keys so far keep that text and stay staged, and the reading doesn't change. With nothing staged, nothing |
| ⌫ | Takes back the last staged key, and the preedit is read again without it. With nothing staged, the app's |
| F keys | Per the F-key bindings (§6.5); by default swallowed, doing nothing |
| ⌥Space | Commits the preedit, then `　` (U+3000, full-width space) |
| 半角/全角 | **full-width**, applied to the staged text (the preedit, which stays staged in that width, typing going on after it) or else the selection |
| ⇧半角/全角 | **half-width**, the same way |

Rules that follow from the table:

- With a preedit, Tab / ⇧Tab cycle; they don't commit-then-tab. The "commit, then act" Tab only
  applies when **[decide]**: never (Tab always cycles when something is staged), or after the
  first cycle step? Proposed: Tab with a preedit always cycles; with none, it goes to the app.
  As built: the proposal.
- With **nothing staged**, Space, Enter, Esc, Tab, arrows and ⇧Enter go to the app unchanged
  (proposed; **[decide]** for ⇧Enter, which the table says is always swallowed). かな with nothing
  staged does nothing. As built: as proposed, ⇧Enter included.
- **Which keys are staged.** A key of the 48-key kana space is staged when a candidate has a meaning
  for it (with ⇧ too; with ⇧⌥ where the layout defines it). Any other key, and ⌥ + a key without ⇧,
  commits the preedit and goes to the app. Two layouts change this for their own keys: with only
  search layouts in use, Space is part of the search and ⇧Space commits and types a space
  ([150](layouts/150-symbols.md)); in bada roman, digits and punctuation commit the word and then
  type themselves ([15](layouts/15-bada-roman.md)).
- While a hanja list is open (bada's `hanja` action, [13](layouts/13-bada.md) §6), the menu keys move
  in it, Enter and Space take the highlighted hanja, and Esc and ⌫ close it.
- Esc **commits** (it doesn't cancel), as the table says. Recorded because most IMEs cancel on
  Esc.
- Cmd / Ctrl shortcuts: commit first, then pass through (as JIS Korean §3.5).

### 6.4 The suggestion menu (negative indexes)

The interpretations of the staged keys form one menu with **signed indexes**:

| Index | Entry |
|---|---|
| … −2, −1 | **Script forms**: the same reading written another way, defined by the method |
| **0** | The top-ranked interpretation: what shows while typing |
| 1, 2, … | The other ranked interpretations: kanji conversions and mixed suggestions, every enabled candidate's readings together |

- Typing lands on **0**. Next (かな, Tab, ↓) goes up from there; previous (⇧かな, ⇧Tab, ↑)
  goes down, and **below 0 into the negative entries**.
- **A picked entry is fixed by typing on** (owner, 2026-11-12; before, 2026-11-11, it could still
  change when a longer entry extended it). After Next has picked an entry above 0, the next key
  fixes it: the keys so far keep the picked text, still staged, and the new keys start after it
  (3 U Q with あなた picked, then more keys: あなた stays). **The layout that was fixed sticks**: its
  reading of the keys after shows first (QWERTY's `inter` picked, then `polate`: `interpolate`;
  `cockpit`'s 콕핏 picked, then `eui`: 콕핏의), until a row is chosen by hand again or the text is
  committed. **⌃かな fixes what shows without picking another row**, the top entry included. ⌫ takes
  back the keys after a fix, then the fixed text a character at a time.
- Japanese methods (20 Kana, 21 Romaji) define **−1 hiragana** and **−2 katakana**: from a fresh
  reading, ⇧Tab once is hiragana, twice katakana.
- **Optional, off by default** (owner, 2026-11-08): Settings → "Script forms below the top entry"
  (`script_forms`). Off, there are no negative entries: ⇧Tab stops at 0, and katakana is an ordinary
  ranked entry among the kanji conversions, the kana a conversion row (§11.2). Hiragana and katakana
  are also F-key actions (§6.5).
- This **replaces F6 / F7** as the way to get hiragana and katakana. JIS Moonshot's F6–F10 (hiragana,
  katakana, kanji, full-width, half-width) are **not** ported as fixed keys; the F keys are reserved
  for §6.5.
- The menu shows the negative entries above entry 0 (a richer list than macOS's candidate window:
  the shell draws it). It shows only when there is more than one entry, five rows at a time, under
  the start of the preedit; it never appears in screen sharing, recordings or screenshots.
- **Notes.** A row may carry a gray note at its right: romaji or the kana reading on Japanese rows
  (§11.3), `typo?` on bada's typo suggestions, the word as typed on bada roman's dictionary rows, the
  name on a search result.
- **One method, at most two entries** (owner, 2026-11-07). Only one enabled method contributes
  negative entries, and it contributes at most two (−1, −2). The slot exists for Japanese script
  override, but any method may offer it; the **user selects which method** in settings ("Negative
  menu entries"). The default, as built: the lowest-id enabled method that offers them (20 Kana before
  21 Romaji). Today only 20 and 21 offer any.
- Other conversions are **not** negative entries: half-width comes from ⇧半角/全角, the old F8–F10
  conversions from F-key actions (§6.5), and the plain English reading is a ranked entry (≥ 0)
  whenever QWERTY (0) is enabled. `en-words.txt` says whether it's a known word: a common word (the
  list's head) or a **rare word** (its `#! rare` section, Webster's Second from macOS's word list:
  `engine/tools/build_en_rare.py`). Beside layouts that have a key model, the key models rank it
  and a known word counts for more (§7.0; owner, 2026-11-11: `interpolate`, not 인텔포라테). Beside
  layouts without one it ranks by the table there: a common word over a Korean reading that's no
  known word, a rare word too, but under a common English word and under any known Korean word;
  other letters last.
- **No wrap.** Next on the last entry and previous on the lowest negative entry do nothing.
- Committing commits the entry shown, whatever its index.

### 6.5 F-key actions

**F6–F10 are reserved by bada ime** (macOS's Japanese input methods already use them for kana
conversion, so apps don't rely on them while a Japanese input source is on).

- **Methods declare actions.** Every henkan method (layout) may publish a list of **F-key actions**:
  named operations on the staged text or the menu. Publishing them is **optional by design**: a
  method with none is valid, and the core never assumes any. The Japanese methods publish the
  usual macOS / MS-IME set:

  | Action (Japanese methods) | Usual key |
  |---|---|
  | `hiragana` | F6 |
  | `katakana` | F7 |
  | `halfwidth-katakana` | F8 |
  | `fullwidth-latin` | F9 |
  | `halfwidth-latin` | F10 |

  The "usual key" is only a hint for a settings preset; nothing is bound by default. Both Japanese
  methods (20 Kana, 21 Romaji) publish these five; Latin is the keys as JIS QWERTY (for Romaji, the
  letters typed). The settings window lists them under each method. Each lasts until the commit.
- **The user binds keys to actions** in settings: each intercepted F key is one of
  - **swallowed** (the default): the key is eaten and does nothing;
  - **passthrough**: the key goes to the app unchanged;
  - **`<method>.<action>`**: that method's action. If the method isn't enabled, or has nothing to
    act on, the key is swallowed.
- **Intercept F keys** (setting, on by default): off, every F key passes through to the app, and no
  binding applies.
- A settings preset "Japanese IME (F6–F10)" binds the five actions above to their usual keys: a
  button on the F Keys tab (it binds 20 Kana's, for the profile being edited).
- **The actions, as built** (the settings window lists each as "method: action"):

  | Method | Actions | Described in |
  |---|---|---|
  | 11, 12 | `commit`, `commit-bare` | §7.0 |
  | 13 bada | `raw`, `prediction-1` … `prediction-5`, `qwerty-fullwidth`, `qwerty-halfwidth`, `hanja` | [layouts/13-bada.md](layouts/13-bada.md) §6 |
  | 15 bada roman | `initials`, `vowels`, `finals`, `fullwidth-latin`, `halfwidth-latin` | [layouts/15-bada-roman.md](layouts/15-bada-roman.md) |
  | 16 bada plain | `qwerty-fullwidth`, `qwerty-halfwidth`, `initials`, `vowels`, `finals`, `commit-bare` | [layouts/16-bada-plain.md](layouts/16-bada-plain.md) |
  | 20, 21 | `hiragana`, `katakana`, `halfwidth-katakana`, `fullwidth-latin`, `halfwidth-latin` | above |
- **Only F6–F10** are intercepted: a carry-over from Japanese input methods. Every other F key
  always goes to the app.
- **Modifiers are up to the method.** The user binds plain F6–F10; what ⇧ / ⌥ / ⌃ + that F key does
  is decided by the method its plain key is bound to (a method may define modified actions, or
  none). Unbound or undefined: swallowed, like the plain key. As built no method defines any, so a
  modified F key bound to an action is swallowed (one bound to passthrough still passes).
- **Muhenkan mode passes F keys through**, always (no preedit, no methods).
- With nothing staged, an action applies to nothing and the key is swallowed (a method may define
  otherwise, e.g. act on the selection). **Staged text comes before the selection** (owner,
  2026-11-11): the two rarely coexist, so an action that can take either (bada's `hanja`, bada
  roman's jamo actions, 半角/全角) takes the staged text, and the selection only when nothing is
  staged.

### 6.6 Henkan profiles (owner, 2026-11-08)

Optional, **off by default**: Settings → Profiles. Three keys can each bring up henkan mode
with its own set of henkan candidates:

| Key | Profile | Default |
|---|---|---|
| かな | `mac-kana` | its own candidates (the enabled set of §5.3) |
| 変換 | `henkan` | linked to `mac-kana` |
| カタカナ | `katakana` | linked to `mac-kana` |

- Each profile has **its own candidates** (an ordered enabled set, e.g. QWERTY + KTS + kana for かな
  and QWERTY + KTS + romaji for 変換) **or links to another profile** and uses its set; any profile
  can link to any other, `mac-kana` included. A loop or a dead end falls back to `mac-kana`'s own set.
- The default setup is `mac-kana` only, the other two linked to it: with profiles off, or on but not
  set up, every key gives the same henkan.
- A profile key in muhenkan mode enters henkan in that profile. In henkan mode, a different profile's
  key commits what's staged and switches profile; the same profile's key is the next menu entry
  (⇧: previous), as かな is (§6.3).
- **かな in another profile** (in henkan mode with `henkan` or `katakana` in use) is a setting **per
  profile** (`kana_in_profile`: `henkan` → …, `katakana` → …), e.g. 変換's profile on the default
  and カタカナ's sending かな straight to muhenkan:

  | Setting | かな does |
  |---|---|
  | `next-or-mac-kana` (**default**) | The next menu entry (⇧かな: previous) when there's a menu to move in (something staged with more than one reading); else back to `mac-kana` |
  | `mac-kana` | Always back to `mac-kana` |
  | `cycle` | The next profile in the cycle order (⇧かな: the previous); the order is a setting, `mac-kana` → `henkan` → `katakana` by default |
  | `henkan` | To `henkan`'s profile (already there: the next menu entry) |
  | `katakana` | To `katakana`'s profile (already there: the next menu entry) |
  | `muhenkan` | Leaves henkan mode, as 英数 |

  Going to another profile commits what's staged.
- **PC keyboards' 変換 and カタカナ** never become key events of their own in macOS: 変換 (HID usage
  0x8A, International4) becomes nothing and カタカナ (0x88, International2) becomes かな. bada ime
  reads them at the HID level (`mac/Sources/HIDKeys.swift`, **needs Input Monitoring**): a recorded
  HID key is stored as `0x70000 | usage`; a かな event right after カタカナ's HID report is taken as
  カタカナ, and 変換, with no event, acts from its HID report.
- 変換 and カタカナ are caught by the same key tap as 英数 and かな, so they **need Accessibility**.
  macOS may not pass a PC keyboard's 変換 and カタカナ on (and some keyboards send かな for
  カタカナ), so Settings **records** which key code each profile key is (through the key tap, so
  any key works wherever focus is; Esc or the button again cancels; Clear forgets it); かな and 英数 can't be recorded. Unrecorded: the profile is only reachable through links.
- **F keys per profile:** each profile but `mac-kana` can have **its own F6–F10** (intercepted or
  not, and each key's binding), or use `mac-kana`'s (the default). F-key actions only act for a
  method among the profile's candidates.
- This settles §6.1's (b) for keys; what HYPER+かな and MEH+かな do with profiles is still open.

### 6.7 Caps Lock (owner, 2026-11-11)

bada ime handles Caps Lock itself, without remapping the key: it reads each press at the HID level
(Input Monitoring), decides what Caps Lock should be and sets it (the system's Caps Lock state, so
its LED and every app follow), and undoes the toggle macOS makes on its own; the key tap
(Accessibility) swallows macOS's own toggle so apps don't see it. Without both permissions Caps Lock
is macOS's. In another input source than bada ime's (ABC in a password field) it's plain Caps Lock.
macOS's "Use Caps Lock to switch to and from ABC" must be off.

The Caps Lock state bada ime sets lights the LED, but macOS's key events don't carry it. So in
muhenkan, while Caps Lock is on, the input method types the capitals itself (a letter key without
⌘ ⌃ ⌥: its capital, as ABC has it), and the two keylayouts have a Caps Lock layer for the Keyboard
Viewer (§4.1).

One setting, three behaviours:

| Setting | Caps Lock | ⇧ ⌃ ⌥ ⌘ + Caps Lock |
|---|---|---|
| **toggle** (default) | The other mode, as MEH+かな (§6.1). The LED shows the Caps Lock state, and blinks once when Caps Lock switched mode | Caps Lock, in muhenkan only, **muhenkan-transient** (below); in henkan nothing |
| **transient** | Caps Lock, **muhenkan-transient**: on in muhenkan, forgotten (off) when muhenkan is left. In henkan it stays off: pressed there it does nothing, and if another app turns it on, bada ime turns it back off | as Caps Lock |
| **respect** | Caps Lock, **login-transient**: henkan mode doesn't turn it off, and it's forgotten only at logout. Henkan layouts compose as usual with it on | as Caps Lock |

What Caps Lock types in muhenkan is the **muhenkan rule** (§6.2): ALL CAPS or UPPER_SNAKE.

When it misbehaves on a keyboard: [troubleshooting.md](troubleshooting.md#caps-lock-spec-67).

## 7. Build and layout of the repo

| Path | What |
|---|---|
| `/engine` | Platform-independent core (Rust, `bada-engine`): layouts, composition, the menu, F-key actions, the user dictionary. Event in → output out, no I/O. C ABI in `include/bada.h` (JSON in and out; bada-compatible v1.1, §9) |
| `/engine/data` | Shipped data: Korean word counts (`ko-words.tsv`, built by `engine/tools/build_ko_words.py`) and the typo model built from them (`ko-jamo-4gram.bin`), English words (`en-words.txt`, with its `#! rare` section), hanja, OECD names, Japanese words and kanji conversion (`ja-words.txt`, `kanji.bin`, `ja-kana-native.txt`, `ja-common-words.txt`), layout 15's English dictionary (`en-ko/`: common words, brand names by country, the order of brands that share a name) and romaji shortcuts (`ja-shortcuts.tsv`), the symbol, emoji and Unicode name lists of 150 and 151 (`unicode-aliases.tsv`: the hand-written colloquial names), with their licenses. The scripts that build them: `/engine/tools` |
| `/mac` | InputMethodKit shell (Swift, no Xcode): the two input modes (§4.1), the 英数 / かな key tap, the HID keys (§6.6) and Caps Lock (§6.7), the menu window, the settings window, the user dictionary's file (§8), the state for other apps (§9), the Keyboard Viewer's layouts (§4.1) |
| `/docs` | This spec; a page per layout, each with its rules, design principles and history, and combination examples (`docs/layouts`, images in `docs/images/layouts`; the Bada layout and its design history: `docs/layouts/13-bada.md`, its saved candidates: `docs/history/`); the typing robot (`robot.md`); the dictionary contract, its schema and an example; the interactive setup prompts (`interactive-prompts/`); `troubleshooting.md`; an example setup (`examples/hhkb-profiles`) |
| `/tools/analysis`, `/tools/render`, `/tools/editor`, `/tools/practice`, `/tools/setup-page` | Corpus analysis and layout optimisation, the layout renderer (with combinations, and `export_png.py` for the docs' images), the layout editor, a key drill, a page of a setup's key legends per mode ([examples/hhkb-profiles](examples/hhkb-profiles/README.md)) |
| `/tools/icon`, `/tools/social-preview` | The app icon's PNGs and the repository's social preview, rendered into `docs/images` |
| `/utils/learner-app` | A local page that logs your typing and exports it for your dictionary (§8), and a copy-practice page |
| `/web` | The web simulator ([bada.sjun.me](https://bada.sjun.me)): the engine compiled to WebAssembly behind a text field, the Mac shell's key handling, menu and settings window ported to the page (settings in the browser's local storage, the same `bada-settings/1` file), and the keyboard drawn by §7.1's renderer. Deployed by `.github/workflows/deploy-web.yml`. `/learn` redirects to Learn Korean, [learn.sjun.me](https://learn.sjun.me) (team-mirae/learn-korean): typing lessons on a copy of this engine: the learner picks bada roman (15), 2-set (10) or bada (13) and learns it by typing; also a public MCP server |

All behaviour lives in the engine; the shell translates keys, switches input sources and applies
the output.

- Engine: `cargo test` in `/engine` (unit tests, plus `tests/real_data.rs` on the shipped data and
  `tests/state.rs` for §9's `staged`). The typing robot, `tests/robot.rs`, runs in release builds
  only: [robot.md](robot.md).
- macOS: `make` in `/mac` builds `build/BadaIME.app` (universal, so Rust needs both the
  `aarch64-apple-darwin` and `x86_64-apple-darwin` targets; it builds against the macOS 26 SDK or
  later, for the menu's glass panel, and runs on macOS 13 or later); `make install` copies it to
  `~/Library/Input Methods`, registers it (enabling both modes on a first install) and adds a login
  agent so the 英数 / かな tap runs from login. Then follow §4.1's setup. `make dist` zips the app
  (what `.github/workflows/mac.yml` builds when a push touches the engine or the shell, and attaches
  to a `v*` tag's release; `mac/tools/install-ci.sh` installs a run's app); `make uninstall` removes
  it; `make SENSITIVE=1` builds layout 15's sensitive words in. `mac/tools/test_profile_keys.sh`
  checks §6.6's language-key decisions.
- Web: `node web/build.mjs` builds `web/dist`, `node web/test/smoke.mjs` types on the engine as
  WebAssembly ([web/README.md](../web/README.md)).
- Bundle id `me.sjun.inputmethod.Bada`, modes `….Muhenkan` and `….Henkan`.
- Diagnostics: `defaults write me.sjun.inputmethod.Bada DiagnosticLog -bool true` turns on a log of
  the input method's own events, `~/Library/Logs/BadaIME.log`
  ([troubleshooting.md](troubleshooting.md)).

### 7.0 What 0.3.1 builds

- Layouts **0** (QWERTY), **10** (2-set), **11** (JIS Korean), **12** (JIS Moonshot), **13** (Bada, layouts/13-bada.md), **14** (3-set JIS), **15** (bada roman), **16** (bada plain, layouts/16-bada-plain.md), **18** (Korean Roman), **19** (Kana Korean),
  **20** (Kana), **21** (Romaji), **150** (Symbols) and **151** (Unicode). 20 and 21 convert to kanji (Mozc's dictionary, §11)
  and, with script forms on, give the negative entries (−1 hiragana, −2 katakana); their menu rows carry romaji / kana notes.
- 11 and 12 are the Korean readings only, deterministic: the keys, implied ㅇ, the double press and
  ⇧ + vowel of JIS Korean §3–§4.2, and JIS Moonshot §9's slots and ⇧ + consonant. Not ported: their
  predictions, hanja, fixed F6–F10, and JIS Korean's ⇧Z as F10 (⇧Z is ㅋ here). Both publish two
  F-key actions (§6.5), bound to nothing by default: `commit` (this layout's reading as typed,
  whatever ranks first) and `commit-bare` (the same with the implied ㅇ dropped, as JIS Korean's F6:
  ㅏ → ㅏ, 가어 → 가ㅓ, 와 → ㅘ; a typed or asked-for ㅇ and a syllable with a final stay).
- Ranking. Every reading gets a score, and the menu is the readings by score, one row per text. A
  layout's own readings are scored by rules over word counts (a known word beats an unknown one):
  the table below. **Between the layouts of a set, key models decide** (after the table). Layout 13
  adds typo suggestions under its reading from a jamo 4-gram model (layouts/13-bada.md §5); it reads
  its keys one way only. Layout 15 uses the same model to choose among readings when none is a
  known word.

  | Reading | Score |
  |---|---|
  | A known Korean word (any Korean layout; counts from `ko-words.tsv` and the user's dictionary) | 100 + ln(count) |
  | A Japanese reading that is a known word (20, 21): the user's own words, then its conversions and the kana, in §11.2's order | 92 and just above |
  | A common English word (0) | 90 |
  | A rare English word (0) | 85 |
  | Korean in whole syllables, but no known word | 80 |
  | A Japanese reading that is no known word, and its conversions | 70 and just above |
  | A search as typed, then what it finds (150, 151) | 60, 59 |
  | Korean with loose jamo left over | 40 |
  | Letters that are no English word (0); a Japanese reading with a stray ゛ ゜ or Latin letters; bada roman's reading of letters after a digit or symbol | 30 |
  | Other QWERTY text (digits, symbols) | 20 |

  The layout's weight (§5.3) multiplies its scores; a tie goes to the layout earlier among the
  candidates. Layouts 15, 18 and 19 rank their own other readings beside this
  ([15](layouts/15-bada-roman.md), [18](layouts/18-korean-roman.md), [19](layouts/19-kana-korean.md)). A
  soft negative word of the user's loses 60; a strong one is left out (§8).

  **In a set, key models compare the layouts** (`engine/src/keymodel.rs`; JIS Moonshot's key model,
  one per layout). `engine/data/key-models.bin` (3.6 MB, built into the engine;
  `engine/tools/build_keymodel.py`) has, for each of 0, 10–16, 18, 20 and 21, a 4-gram model of the
  keys pressed when its language is typed on it: the 48 keys, the same with Shift, and the word
  boundary (Witten–Bell smoothed). When two or more enabled layouts with a model read the staged
  keys, each scores them as the start of a word that ends at the last key or (counted 0.02) goes
  on, and the scores become shares that sum to one. A key typed with ⌥ is the same character in
  every layout, so the models don't see it. Each such layout's rows then move together, keeping
  their order and the gaps between them: its first row to **80**, **85** if the table knows the
  reading as a word (85 and up), **40 more** for a word of the user's dictionary; times the
  layout's weight; plus **5 × ln(share)**, at most −60. So the table orders one layout's rows, and
  the keys say which layout was meant: `the` shows beside bada, not 판; `kareha` is 彼は and
  `hanguk` 한국 with 15 and 21. A layout alone, a layout without a model (19, 150, 151) and any
  layout beside only such layouts keep the table's scores.

  **A word and its particle** (JIS Moonshot §5.3; `engine/src/particles.rs` has its lists). With
  QWERTY in a set, one staged run may be an English word and then a particle typed on another
  layout of the set: a Korean particle or ending on 10–14 or 16 (Claude에, report를, client에서;
  two or three stacked: client들에게), or a Japanese one on 20 or 21 after a capitalised name
  (GitHubで). The start is a known English word or a capitalised name, and the whole isn't an
  English word itself (`then` stays `then`). The key models score it: the start under QWERTY's,
  the particle under its layout's, each as a word that ends, times 0.5 after a known word and 0.05
  after a name; it ranks as a known word with that share among the readings of all the keys. A
  particle of one key after a name is offered, not shown (Claude and K: `Claudek`, then Claudeの).
  Not built from Moonshot: staged phrases (§5.5 there), and 15 and 18, whose letters are QWERTY's.

  On the typing robot's layout sets ([robot.md](robot.md#layout-sets): every set of two or three
  production layouts of different languages) the first row is right 93.9% of the time, and each
  layout by itself gives 94.3%. The table alone gave 82.3% (on the 37 sets there were before
  layout 16).
- HYPER+かな and MEH+かな both switch to the other mode: with one muhenkan layout and one henkan
  there is nothing else to go back to or cycle through. The user cycle order (§5.3) isn't in the
  settings window yet.
- Default settings: henkan candidates = [16] (bada plain; [13] before 2026-11-12, and settings already saved keep their candidates); muhenkan layout 0; F6–F10 intercepted and swallowed,
  nothing bound; profiles and script forms off; every weight 1; layout 15's romaji shortcuts on; Caps
  Lock toggle, ALL CAPS.
- The web simulator (`/web`) runs the same engine and the same settings; what the page adds or
  lacks is in [web/README.md](../web/README.md).

### 7.1 Keyboards drawn by the renderer

Both have Control left of A. Bottom rows (1u each but the space):

- **HHKB JIS** (15u): fn · 0.25u gap · 半角/全角 · ⌥ · ⌘ · 英数 · space (2.5u) · かな · ⌘ · ⌥ · fn ·
  0.25u gap · ← ↓ →. ↑ sits in the Shift row, left of right Shift.
- **Apple-style JIS** (14.5u, MacBook): caps · ⌥ · ⌘ · 英数 · space (4.5u) · かな · ⌘ · 🌐 · ← ↓ →, with
  ↑ stacked over ↓ at half height; no right ⌥. The left edge is 0.5u shorter than a full board's:
  no 半角/全角 key (1 is 1.5u), tab 1u, control 1.25u, left shift 1.75u.

## 8. The user dictionary

`~/Library/Application Support/bada-ime/dictionary.json`: positive words and negative words (soft or
strong), for every layout. The contract is [dictionary-contract.md](dictionary-contract.md), with a JSON Schema and an example; the
interactive setup is [interactive-prompts/](interactive-prompts/README.md). Loaded at launch and on
the next activation after the file changes; the input menu's "Open Dictionary Folder" opens its
folder, and Settings → Dictionary says what was loaded, or why it wasn't. One dictionary serves the
whole process (every text field's engine). `utils/learner-app` builds one from your own typing.

## 9. State for other apps (bada-compatible v1.1)

The engine exports the optional `bada_state` (`engine/include/bada.h`), bada-compatible v1.1's
`P_state` ([bada-compatible-v1.md §12](https://github.com/team-mirae/bada-compatible/blob/main/docs/bada-compatible-v1.md)):
read-only, it changes nothing. Its `staged` is exactly what `bada_flush` would commit now, in either
mode (the ⌥ + letter variants: what shows; a menu on the host's selection: `""`); `Engine::flush`
commits `Engine::staged`, so the two can't differ. The display fields are the last Output's.

The macOS shell publishes it for scripts (Hammerspoon, Karabiner-Elements, a status bar), read-only:
nothing read from outside flows back into the engine.

- **File** `~/Library/Application Support/me.sjun.inputmethod.Bada/state.json`, rewritten atomically
  (a temporary file, then a rename).
- **Distributed notification** `me.sjun.inputmethod.Bada.state`, its object the same JSON.
- **When:** after every engine call that returns an Output, and on mode, profile, focus, input
  source and Caps Lock changes. Only when the JSON changed.
- **Fields:** the engine's (`format` `"bada-compatible-state/1"`, `mode`, `profile`, `staged`,
  `preedit`, `menu`, `selected`, `zero`, `horizontal`, `need_selection`) as is, and the shell's under
  `host`: `app` (the focused app's bundle id), `secure`, `input_source` (the selected input source's
  id), `active` (one of bada ime's modes is selected), `caps_lock` (`on` as held, §6.7; `handled`,
  `setting`, `rule`) and `version`.
- **Secure input** (a password field): nothing typed is published: `staged`, `preedit` and `menu`
  are empty, and `host.secure` is true.

```lua
-- Hammerspoon: the mode in the menu bar
local bar = hs.menubar.new()
local function show(json)
  local s = hs.json.decode(json)
  if s then bar:setTitle(s.mode == "henkan" and "가" or "A") end
end
watcher = hs.distributednotifications.new(function(_, json) show(json) end, "me.sjun.inputmethod.Bada.state")
watcher:start()
```

Karabiner-Elements: a script (run from Hammerspoon on the notification, or a launch agent watching the
file) reads the file and sets a variable for rules to match, e.g. `'/Library/Application
Support/org.pqrs/Karabiner-Elements/bin/karabiner_cli' --set-variables '{"bada_mode": "henkan"}'`.

## 10. Open questions

Collected from above:

1. §4.3 scoring of candidates without a key model (as built: their rule scores, §7.0).
2. ~~§5.2 whether Bada is deterministic~~: deterministic (owner, 2026-11-08).
3. §6.1 what "most recently used henkan" and MEH+かな's sources are: (a), (b) or (c).
4. §6.2 半角/全角 with no selection in muhenkan.
5. §6.3 Tab: cycle versus commit-then-tab; keys with nothing staged.
6. ~~§6.4 negative entries, menu ends~~: settled 2026-11-07.
7. ~~§6.5 F keys~~: settled 2026-11-07.

## 11. Japanese (layouts 20 and 21)

What JIS Kana ([20](layouts/20-kana.md)) and JIS Romaji ([21](layouts/21-romaji.md)) have in common:
how their readings rank, kanji conversion and the menu's notes. Their keys and spellings are on their
pages; their script forms are in §6.4 and their F-key actions in §6.5. Code: `engine/src/japanese.rs`;
script forms and F-key actions in `engine/src/engine.rs`.

### 11.1 Ranking

A reading is a known word when it's in `engine/data/ja-words.txt` (JIS Moonshot's hand-written
list), a kana-native word, or a Japanese word in the user's dictionary; one with a stray ゛ ゜ or
pending Latin letters scores low. The scores are in §7.0's table: they order a Japanese layout's
rows, and rank it beside a layout that has no key model. Beside the other layouts the key models
decide (§7.0): with 13 and 21 enabled, `konnichiha` shows こんにちは and D H G R H V H B E M A H
still shows 감사합니다. A word of the user's dictionary counts in whatever script it's written,
hiragana, katakana or kanji (§11.2).

### 11.2 Kanji conversion

Both layouts convert their kana reading to kanji (`engine/src/kanji.rs`, ported from JIS Moonshot:
a word lattice over the kana, word costs plus part-of-speech connection costs, at most 40 kana).
The data is `engine/data/kanji.bin` (11 MB), built by `engine/tools/kanji/build_kanji.py` from
**Mozc's open-source dictionary** (pinned commit); notices in `engine/data/LICENSE-mozc.txt`.
On Mozc's evaluation set (`engine/tests/data/kanji-evaluation.tsv`): 69% exact, 79% in the top 3.

- The menu (§6.4) is **0 and forward: the conversions, cheapest first, with the kana among
  them**; with script forms on, also **−1 hiragana, −2 katakana**. So typing shows the top
  conversion (わたしはがくせいです → 私は学生です), Tab the other conversions.
- **Where the kana goes** (owner, 2026-11-08): where conversion ranks it; a known word's kana
  (`ja-words.txt`, or kana-native) no lower than second (ぼく: 僕 may come first, ぼく next); a
  **kana-native** word's kana first. Otherwise last.
- **Kana-native words** (`engine/data/ja-kana-native.txt`, built into the engine): about 870 words
  written in kana at least 60% of the time in Tatoeba's Japanese sentences (この, その, こと, もの,
  あなた, ちょっと, ありがとう), counted by `engine/tools/build_kana_native.py` with the robot's test
  sentences left out. Conversion treats their kana as one below their cheapest kanji, so phrases
  keep them in kana too (このほん → この本, not 個の本). The cost: Mozc's evaluation set lost 5 exact
  sentences of 613 (431 → 426).
- **Common words** (`engine/data/ja-common-words.txt`, built into the engine): the 3,000 most common
  words of Tatoeba's Japanese sentences not written in kana (一, 死, 原因, ケン), with their readings,
  from the same script. When the whole reading is one, its three most common words come under
  the first conversion and a known word's kana (ひ: 日, ひ, 火): conversion of a lone short word ranks them low (いち → 1, し → し), and
  the typing robot's `production` bar needs every common word in the first 5 rows (robot.md).
  Phrases and sentences are converted as before: the lattice's costs aren't changed, so に stays
  a particle in 東京に.
- **The user's words** ([dictionary-contract.md](dictionary-contract.md)) come first: a positive
  Japanese word that is the kana typed, its katakana or one of its conversions (looked for among the
  first 32, further down than the menu lists) or common words. Of several, the most used first.
- Up to six conversions; one that is just the kana isn't repeated, nor its katakana with script
  forms on.
- Unknown or broken readings (a stray ゛, pending Latin letters) aren't converted.

### 11.3 Menu notes (owner, 2026-11-07)

Each Japanese row carries a gray note at its right: **kana rows their romaji** (Hepburn as IMEs spell
it: らーめん ra-men, がっこう gakkou, しんよう shin'you; `engine/src/romaji.rs`), **kanji rows their
kana reading** (私は学生です わたしはがくせいです). Japanese rows are drawn in Hiragino Sans, so kanji
take their Japanese forms.
