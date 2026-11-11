# Dictionary contract: your own words for bada ime

bada ime ranks what it reads from your keys with built-in word counts. This file adds your own: the
words you mean and the words you never mean. It applies to every layout: positive words rank a
reading up, negative words rank it down or leave it out.

`docs/dictionary.schema.json` is its JSON Schema; `docs/dictionary-example.json` is a worked
example. [utils/learner-app](../utils/learner-app/README.md) learns entries from your typing. The interactive setup ([interactive-prompts/dictionary.md](interactive-prompts/dictionary.md))
builds one with you.

## The file

```json
{
  "format": "bada-dictionary",
  "version": 1,
  "source": { "tool": "Claude Code", "created": "2026-11-07", "description": "from my notes, round 1" },
  "positive": [
    { "word": "데이터센터", "count": 40 },
    { "word": "감사합니다", "confidence": "confirmed" },
    { "word": "kubernetes", "language": "english" },
    { "word": "サーバー", "language": "japanese" }
  ],
  "negative": [
    { "word": "감소합니다", "strength": "strong" },
    { "word": "god", "language": "english" }
  ]
}
```

**Required:** `format` (`"bada-dictionary"`) and `version` (`1`). Everything else is optional, and
fields bada doesn't know are ignored, so later versions can add some. A file with the wrong format
or version, or that isn't valid JSON, isn't loaded at all; the settings window says why.

### `positive`: words you mean

| Field | | |
|---|---|---|
| `word` | required | The word as you commit it: Hangul for Korean, Latin for English, kana or kanji for Japanese |
| `language` | `korean` (default), `english`, `japanese` | Which readings it counts for |
| `count` | integer ≥ 1 | How often you use it. Added to the built-in count |
| `confidence` | `tentative` 10, `likely` 50 (default), `confirmed` 200 | Used as the count when there's no `count` |

What a positive word does, as built:

- **Korean:** it counts as a known word in every Korean layout, its count added to the built-in one.
- **English:** QWERTY's reading of it ranks as a common English word, whatever its case.
- **Japanese:** it counts when what you type in Kana or Romaji writes the word: the kana itself
  (さーばー), its katakana (サーバー) or one of its conversions (kanji: 佐藤 for さとう; looked for
  among the first 32 conversions, further down than the menu lists). The reading then ranks as a
  known word, and your word comes first in the menu; of several words with one reading, the most
  used first. The file has no readings, so a kanji word that conversion never makes of what you
  typed can't be offered.

A word listed twice adds its counts.

### `negative`: words you never mean

| Field | | |
|---|---|---|
| `word` | required | The reading you don't want |
| `language` | `korean` (default), `english`, `japanese` | Which layouts' readings it applies to (English: QWERTY; Japanese: Kana and Romaji, kanji included) |
| `strength` | `soft` (default), `strong` | `soft`: ranked far down, still in the menu. `strong`: never offered, as a typo suggestion either |

### `source` (optional)

Who wrote the file and when: shown to no one, kept for you.

### Limits

20,000 positive and 20,000 negative; entries past a limit are ignored. Keep the file
under 2 MB.

## Where it goes

`~/Library/Application Support/bada-ime/dictionary.json`. The input menu's **Open Dictionary
Folder** (and the button in bada ime Settings) opens the folder. It's loaded at launch and again
the next time bada is activated in a text field after the file changed (switch away and back).
**Undo:** delete or rename the file; it's dropped on the next activation.

The web simulator ([bada.sjun.me](https://bada.sjun.me)) takes the same file in Settings →
Dictionary and keeps it in the browser.

## Privacy

The file holds words, not sentences: no names of people, addresses, account numbers or anything you
wouldn't want in a word list. It stays on your Mac; bada never sends it anywhere.

## For AI agents

- Build the file **with** the user, from words they name or files they let you read
  ([interactive-prompts/rules.md](interactive-prompts/rules.md)). Read for words, never copy
  sentences, and leave out anything private.
- Prefer real counts (from their writing) over `confidence`.
- **Negative** words: `soft` unless the user asks for `strong`.
- Show the whole file before writing it, validate it against `docs/dictionary.schema.json`, and ask
  before replacing an existing one (offer a merge).
