"""Appends the rare English tier to engine/data/en-words.txt.

    python3 engine/tools/build_en_rare.py [--words /usr/share/dict/words]

Source: macOS's /usr/share/dict/words, Webster's Second International (1934, public domain).
Kept: lowercase, ASCII letters only, 5 letters or more, not already in the common list above the
`#! rare` line. The engine ranks these under common English words and under any known Korean word,
over a reading that is no known Korean word (interpolate, not 인텔포라테). Re-running replaces the
section.
"""
import argparse, os

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "data", "en-words.txt")
MARK = "#! rare"

ap = argparse.ArgumentParser()
ap.add_argument("--words", default="/usr/share/dict/words")
a = ap.parse_args()

head = open(OUT, encoding="utf-8").read().split("\n" + MARK)[0].rstrip("\n")
# A line of the common list is a word, with its count after a tab in the generated part.
common = {l.split("\t")[0].strip().lower() for l in head.split("\n") if l.strip() and not l.startswith("#")}
rare = sorted({w for w in (l.strip() for l in open(a.words, encoding="utf-8"))
               if w.isascii() and w.isalpha() and w.islower() and len(w) >= 5 and w not in common})
with open(OUT, "w", encoding="utf-8") as f:
    f.write(head + "\n" + MARK + ": Webster's Second International (1934, public domain), from macOS's /usr/share/dict/words;\n")
    f.write("# lowercase, 5+ letters, not above. Built by tools/build_en_rare.py.\n")
    f.write("\n".join(rare) + "\n")
print(f"{len(rare)} rare words appended to {os.path.relpath(OUT)}")
