"""engine/data/key-models.bin: the key models of the layout-set ranker (engine/src/keymodel.rs).

One key n-gram model per layout that can share a set (0, 10–16, 18, 20, 21): how likely a run of
keys is when that layout's language is typed on it. Two steps:

1. This script lists what is typed, with counts, in engine/tools/cache/keymodel/lists.tsv
   (language, source, kind, split, text, count):
   - English: Tatoeba's English sentences, the Leipzig Corpora Collection's English sentences
     (derived/leipzig-corpora/en.txt) and 17 U.S.C. (engine/tools/cache/robot/us-title17.htm), as
     words with their capitals, digits and apostrophes (kind `word`: the runs the typing robot
     types) and as the pieces between spaces that carry other punctuation (kind `token`: `(b)`,
     `word,`); and every word of engine/data/en-words.txt once (source `list`), so the letters of
     a word no sentence has still look English.
   - Japanese: Tatoeba's Japanese sentences and 日本国憲法 / 民法 (cache/robot/jp-*.xml), read by
     SudachiPy into phrases as engine/tools/build_robot_corpus.py does (kind `phrase`), a phrase
     with the 、 or 。 after it (kind `punct`), and content words in their dictionary form (kind
     `word`).
   - Korean: 대한민국헌법 / 민법 (cache/robot/ko-*.txt), as runs of Hangul (kind `word`); the
     Korean word counts themselves are engine/data/ko-words.tsv, read by step 2.
   Every sentence listed in engine/tools/cache/robot-sets/exclude-sentences.txt is left out: the
   sentences the typing robot's layout sets are measured on (docs/robot.md), the held-out ones
   included, one per line. BADA_EXCLUDE names another file. One sentence in HELD of each source
   goes to split `test`, for the accuracy step 2 reports.
2. `cargo run --release --example keymodel` (engine/examples/keymodel.rs) types every item on its
   layouts, counts the key n-grams and writes engine/data/key-models.bin.

    tools/analysis/.venv/bin/python engine/tools/build_keymodel.py [--lists-only]

Reads ~/Projects/jis-moonshot/corpora (BADA_CORPORA) and engine/tools/cache/robot (BADA_CACHE).
Deterministic: the split is by CRC of the sentence, the output sorted.
"""
import bz2, collections, html, os, re, subprocess, sys, zlib

HERE = os.path.dirname(os.path.abspath(__file__))
CORPORA = os.path.expanduser(os.environ.get("BADA_CORPORA", "~/Projects/jis-moonshot/corpora"))
CACHE = os.path.expanduser(os.environ.get("BADA_CACHE", os.path.join(HERE, "cache")))
OUT = os.path.join(HERE, "cache", "keymodel")
HELD = 20
# Readings SudachiDict gets wrong for everyday typing (as build_robot_corpus.py's ja_common).
READING_FIX = {"私": "わたし"}
INDEPENDENT = {"名詞", "代名詞", "動詞", "形容詞", "形状詞", "副詞", "連体詞", "接続詞", "感動詞", "接頭辞"}
# What people type for the typographic characters of running text.
PLAIN = str.maketrans({"’": "'", "‘": "'", "“": '"', "”": '"', "–": "-", "—": "-", "…": "."})
# Characters JIS QWERTY types (engine/src/layout.rs, `jis_char`).
TYPED = set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!\"#$%&'()=~|^¥@`[{;+:*]},<.>/?\\_-")


def split_of(sentence):
    return "test" if zlib.crc32(sentence.encode("utf-8")) % HELD == 0 else "train"


def excluded():
    path = os.environ.get("BADA_EXCLUDE") or os.path.join(HERE, "cache", "robot-sets", "exclude-sentences.txt")
    if not os.path.exists(path):
        sys.exit(f"{path}: the benchmark's sentences (one per line), to leave out. BADA_EXCLUDE names another file")
    return set(open(path, encoding="utf-8").read().split("\n"))


def tatoeba(lang):
    for line in bz2.open(os.path.join(CORPORA, f"{lang}_sentences.tsv.bz2"), "rt", encoding="utf-8"):
        parts = line.rstrip("\n").split("\t")
        if len(parts) == 3:
            yield parts[2]


def sentences(text, end=r"(?<=[.;:])\s+"):
    """As build_robot_corpus.py splits its law texts, so the benchmark's sentences match exactly."""
    out = []
    for para in text.split("\n"):
        para = re.sub(r"\s+", " ", para).strip()
        out += [s.strip() for s in re.split(end, para) if len(s.strip()) > 20]
    return out


def us_law():
    raw = open(os.path.join(CACHE, "robot", "us-title17.htm"), encoding="utf-8").read()
    raw = re.sub(r"(?s)<(script|style)[^>]*>.*?</\1>", "", raw)
    raw = re.sub(r"(?i)<(br|p|div|h\d)[^>]*>", "\n", raw)
    text = html.unescape(re.sub(r"<[^>]+>", " ", raw))
    return [s for s in sentences(text) if re.search(r"[a-z]{3}", s) and not s.startswith("(Pub.")]


def wikitext(path):
    t = open(path, encoding="utf-8").read()
    t = re.sub(r"(?s)\{\{.*?\}\}", "", t)
    t = re.sub(r"(?s)<ref.*?</ref>|<[^>]+>", "", t)
    t = re.sub(r"\[\[(?:[^|\]]*\|)?([^\]]*)\]\]", r"\1", t)
    return re.sub(r"'''?|^[=*:#]+|=+$", "", t, flags=re.M)


def ko_law():
    text = "\n".join(wikitext(os.path.join(CACHE, "robot", f"ko-{name}.txt")) for name in ("constitution", "civil"))
    return [s for s in sentences(text, r"(?<=다\.)\s+") if re.search(r"[가-힣]{2}", s)]


def jp_law():
    import xml.etree.ElementTree as ET
    out = []
    for name in ("jp-constitution.xml", "jp-civil.xml"):
        root = ET.parse(os.path.join(CACHE, "robot", name)).getroot()
        out += ["".join(s.itertext()).strip() for s in root.iter("Sentence")]
    return [s for s in out if len(s) > 8]


def leipzig():
    with open(os.path.join(CORPORA, "derived", "leipzig-corpora", "en.txt"), encoding="utf-8") as f:
        for line in f:
            yield line.rstrip("\n")


def english(rows, source, lines, skip):
    counts = collections.Counter()
    for s in lines:
        if s in skip:
            continue
        split = split_of(s)
        plain = s.translate(PLAIN)
        for w in re.findall(r"[A-Za-z0-9']+", plain):
            w = w.strip("'")
            if w:
                counts[("word", split, w)] += 1
        for t in plain.split():
            # A piece with punctuation of its own, every character typed by a key.
            if not re.fullmatch(r"[A-Za-z0-9']+", t) and re.search(r"[A-Za-z0-9]", t) and set(t) <= TYPED:
                counts[("token", split, t)] += 1
    rows += [("en", source, kind, split, text, n) for (kind, split, text), n in counts.items()]


def word_list(rows):
    seen = set()
    for line in open(os.path.join(HERE, "..", "data", "en-words.txt"), encoding="utf-8"):
        w = line.split("\t")[0].strip()
        if w and not w.startswith("#") and w not in seen:
            seen.add(w)
            rows.append(("en", "list", "word", split_of(w), w, 1))


def hira(kata):
    return "".join(chr(ord(c) - 0x60) if "ァ" <= c <= "ヶ" else c for c in kata)


def japanese(rows, source, lines, skip):
    """Phrases as build_robot_corpus.py's `phrases` makes them (a content word and what attaches to
    it), each also with the 、 or 。 that follows it, and content words in their dictionary form."""
    from sudachipy import Dictionary, SplitMode
    tok = Dictionary(dict="core").tokenizer()
    counts = collections.Counter()
    for line in lines:
        if line in skip:
            continue
        split = split_of(line)
        cur = None
        for m in tok.tokenize(line, SplitMode.C):
            pos = m.part_of_speech()
            surface, reading = m.surface(), hira(m.reading_form())
            if pos[0] in ("補助記号", "空白") or not re.fullmatch(r"[ぁ-んー]+", reading):
                if cur:
                    counts[("phrase", split, cur[0])] += 1
                    if surface in ("、", "。"):
                        counts[("punct", split, cur[0] + surface)] += 1
                cur = None
                continue
            if pos[0] in INDEPENDENT and pos[0] != "接頭辞" and surface == m.dictionary_form() and not re.search(r"[0-9０-９]", surface):
                counts[("word", split, READING_FIX.get(surface, reading))] += 1
            starts = pos[0] in INDEPENDENT and pos[1] != "非自立可能" and not (cur and cur[1] == "接頭辞")
            if cur is None or starts:
                if cur:
                    counts[("phrase", split, cur[0])] += 1
                cur = [reading, pos[0]]
            else:
                cur[0] += reading
                cur[1] = pos[0]
        if cur:
            counts[("phrase", split, cur[0])] += 1
    rows += [("ja", source, kind, split, text, n) for (kind, split, text), n in counts.items()]


def korean(rows, source, lines, skip):
    counts = collections.Counter()
    for s in lines:
        if s not in skip:
            for w in re.findall(r"[가-힣]+", s):
                counts[(split_of(s), w)] += 1
    rows += [("ko", source, "word", split, w, n) for (split, w), n in counts.items()]


def main():
    skip = excluded()
    rows = []
    english(rows, "tatoeba", tatoeba("eng"), skip)
    english(rows, "leipzig", leipzig(), skip)
    english(rows, "us-law", us_law(), skip)
    word_list(rows)
    japanese(rows, "tatoeba", tatoeba("jpn"), skip)
    japanese(rows, "law", jp_law(), skip)
    korean(rows, "law", ko_law(), skip)
    rows.sort()
    os.makedirs(OUT, exist_ok=True)
    with open(os.path.join(OUT, "lists.tsv"), "w", encoding="utf-8") as f:
        for row in rows:
            f.write("\t".join(str(x) for x in row) + "\n")
    print(f"{len(rows)} rows → {os.path.join(OUT, 'lists.tsv')}")
    if "--lists-only" not in sys.argv:
        subprocess.run(["cargo", "run", "--release", "--example", "keymodel"], cwd=os.path.join(HERE, ".."), check=True)


if __name__ == "__main__":
    main()
