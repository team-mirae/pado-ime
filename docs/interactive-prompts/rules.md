# Rules for every interactive prompt

These apply from the first question to the last step. If a step in another prompt seems to
conflict with them, these win.

## 1. Ask before reading anything

Before any command, file read or query on the user's system, even a read-only one, say what you'd
run and why, and offer exactly these choices:

> I'd like to check *what*, by running `the command`. It only reads; nothing changes.
>
> 1. Yes, this once
> 2. Yes, and allow read-only checks for the rest of this session
> 3. No: tell me how to check it myself

- **1:** run that one. Ask again for the next.
- **2:** run read-only checks without asking again, this session only; still say what each is for.
  Changes (section 3) are still asked one by one.
- **3:** give the command or the clicks, say what to look for, and wait for the answer.

Read-only means it can't change anything: no `sudo`, no writes, no installs, no settings.

Reading the user's own writing for the dictionary is a read like any other: ask per file or folder.
Exception: the look at the clone in [start.md](start.md) §0, which the user asked for by pointing you
there.

When you report what you found, leave out serial numbers, network addresses, user and computer names.

## 2. Ask before installing anything, one tool at a time

Ask separately for each tool and wait for the answer before the next. Say what it is, why it's
needed, where it comes from, whether it needs a password or a permission, and how to remove it. A
"no" is final for that tool. Don't suggest tools the user didn't ask about.

## 3. Ask before changing anything

One change per question, saying what it changes and how to undo it: installing bada ime, input
sources, Accessibility, Input Monitoring, login items, bada ime's settings, and writing
`dictionary.json`. A change that
needs `sudo` or an administrator prompt is one the user runs: give them the command.

## 4. Everything else

- One question at a time, with numbered choices where the answers are known.
- Use what the user already told you; don't ask again.
- If a step fails, show the error, say what it means, and offer the next option.
- [docs/spec.md](../spec.md) and [docs/dictionary-contract.md](../dictionary-contract.md) are the
  source of truth; if a prompt disagrees with them, follow them and say so.
- At the end, summarise what was installed and changed, and how to undo each item.
