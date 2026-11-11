"""Write layouts.js for the renderer: the catalog's layouts.

Classes: i initial, f final, c consonant (initial or final), v vowel, y syllable, k kana,
l latin, d digit, s symbol, x function.
"""
import json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
JIS = {"Digit1":"1!","Digit2":"2\"","Digit3":"3#","Digit4":"4$","Digit5":"5%","Digit6":"6&","Digit7":"7'","Digit8":"8(",
       "Digit9":"9)","Digit0":"0","Minus":"-=","Equal":"^~","IntlYen":"¥|","BracketLeft":"@`","BracketRight":"[{",
       "Semicolon":";+","Quote":":*","Backslash":"]}","Comma":",<","Period":".>","Slash":"/?","IntlRo":"\\_"}
LETTERS = "QWERTYUIOPASDFGHJKLZXCVBNM"
VOWELS = set("ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ")

def cls(t):
    if not t: return "x"
    if t.isdigit(): return "d"
    if t in VOWELS: return "v"
    if "ㄱ" <= t[0] <= "ㅎ": return "c"
    if "가" <= t[0] <= "힣": return "y"
    if "぀" <= t[0] <= "ヿ": return "k"
    if t.isalpha() and t.isascii(): return "l"
    return "s"

def lay(id_, name, display, kind, table, note=""):
    keys = {}
    for code, (b, s) in table.items():
        keys[code] = {"base": {"t": b, "c": cls(b)} if b else None, "shift": {"t": s, "c": cls(s)} if s else None}
    return dict(id=id_, name=name, display=display, kind=kind, note=note, keys=keys)

def jis_symbols():
    return {c: (v[0], v[1] if len(v) > 1 else "") for c, v in JIS.items()}

def qwerty():
    t = jis_symbols()
    for c in LETTERS: t["Key"+c] = (c.lower(), c)
    return t

TWO_SET = dict(zip(LETTERS, "ㅂㅈㄷㄱㅅㅛㅕㅑㅐㅔ" "ㅁㄴㅇㄹㅎㅗㅓㅏㅣ" "ㅋㅌㅊㅍㅠㅜㅡ"))
TWO_SET_ORDER = "QWERTYUIOPASDFGHJKLZXCVBNM"
def kts():
    t = jis_symbols()
    shift = {"Q":"ㅃ","W":"ㅉ","E":"ㄸ","R":"ㄲ","T":"ㅆ","O":"ㅒ","P":"ㅖ"}
    for c in LETTERS: t["Key"+c] = (TWO_SET[c], shift.get(c, TWO_SET[c]))
    return t

def legacy_korean():   # JIS Korean Mode A (jis-korean docs/spec.md §3)
    t = kts()
    for d, j in zip("1234567890", "ㅃㅉㄸㄲㅆㅟㅝㅚㅢㅘ"): t["Digit"+d] = (j, d)
    t.update({"Minus":("!","?"),"Equal":("(",")"),"IntlYen":("—","^"),"BracketLeft":("'","\""),"BracketRight":("[","「"),
              "Semicolon":("~","-"),"Quote":(":",";"),"Backslash":("]","」"),"Comma":(",",","),"Period":(".","."),
              "Slash":("·","·"),"IntlRo":("\\","_"),"KeyZ":("ㅋ","ㅋ"),"KeyY":("ㅛ","ㅛ"),"KeyU":("ㅕ","ㅕ"),"KeyI":("ㅑ","ㅑ")})
    return t

def legacy_moonshot():  # JIS Moonshot Mode A (jis-moonshot docs/moonshot.md §9)
    t = legacy_korean()
    for d, j in zip("12345", "ㅃㅉㄸㄲㅆ"): t["Digit"+d] = (j, d)
    t.update({"Minus":("ㅙ","왜"),"Equal":("^","~"),"IntlYen":("¥","|"),"BracketLeft":("@","`"),"BracketRight":("[","{"),
              "Semicolon":("ㅖ","예"),"Quote":("ㅒ","얘"),"Backslash":("]","}"),"Comma":("ㅢ",","),"Period":(".","."),
              "Slash":("/","?"),"IntlRo":("\\","_")})
    syl = lambda j, v: chr(0xAC00 + ("ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ".index(j)*21 + "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ".index(v))*28)
    for c in LETTERS:
        j = TWO_SET[c]
        t["Key"+c] = (j, syl("ㅇ", j) if j in VOWELS else syl(j, "ㅏ"))
    return t

def kana():
    k = dict(Digit1="ぬ",Digit2="ふ",Digit3="あぁ",Digit4="うぅ",Digit5="えぇ",Digit6="おぉ",Digit7="やゃ",Digit8="ゆゅ",Digit9="よょ",
             Digit0="わを",Minus="ほ",Equal="へ",IntlYen="ー",KeyQ="た",KeyW="て",KeyE="いぃ",KeyR="す",KeyT="か",KeyY="ん",KeyU="な",
             KeyI="に",KeyO="ら",KeyP="せ",BracketLeft="゛",BracketRight="゜「",KeyA="ち",KeyS="と",KeyD="し",KeyF="は",KeyG="き",
             KeyH="く",KeyJ="ま",KeyK="の",KeyL="り",Semicolon="れ",Quote="け",Backslash="む」",KeyZ="つっ",KeyX="さ",KeyC="そ",
             KeyV="ひ",KeyB="こ",KeyN="み",KeyM="も",Comma="ね、",Period="る。",Slash="め・",IntlRo="ろ")
    return {c: (v[0], v[1] if len(v) > 1 else "") for c, v in k.items()}

DOCS = os.path.join(HERE, "../../docs/layouts")

def syl(ini, vowel):
    I = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ"; V = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ"
    return chr(0xAC00 + (I.index(ini) * 21 + V.index(vowel)) * 28)

def cell_of(tokens):
    """Editor tokens on one key → a renderer cell. Initial and final on one key show as one consonant."""
    if not tokens:
        return None
    texts = []
    for t in tokens:
        if t["t"] not in texts:
            texts.append(t["t"])
    roles = {t["r"] for t in tokens}
    if len(texts) == 2 and [t["r"] for t in tokens] == ["i", "f"]:
        # Interlaced: one key, an initial or a final by the composer's state (ㅃ / ㅄ). Not a ⇧ legend.
        return {"t": "/".join(texts), "c": "c", "r": ["i", "f"]}
    c = "c" if roles >= {"i", "f"} or "c" in roles else ("v" if roles == {"v"} else ("i" if roles == {"i"} else ("f" if roles == {"f"} else ("y" if all(len(x) == 1 and "\uac00" <= x <= "\ud7a3" for x in texts) else cls(texts[0])))))
    return {"t": "/".join(texts), "c": c}

def bada13():
    """Layout 13 as the engine types it (engine/src/layout.rs, docs/layouts/13-bada.md)."""
    d = json.load(open(os.path.join(DOCS, "13-bada.json")))
    keys = {}
    for code in list(JIS) + ["Key"+c for c in LETTERS]:
        v = d["keys"].get(code, {"base": [], "shift": []})
        base = cell_of(v["base"])
        shift = cell_of(v["shift"])
        if shift is None and not code.startswith("Key"):
            shift = {"t": JIS[code][0], "c": cls(JIS[code][0])}        # ⇧ = the key's JIS character
        keys[code] = {"base": base, "shift": shift}
    return dict(id=13, name="JIS Bada", display="Korean (bada)", kind="henkan",
                note="Deterministic: the same keys always give the same text. Blue: initial or final (a final until a vowel takes it); "
                     "orange: final only (never moves); green: vowel. ㅇ is typed (F). Vowels join as in 2-set (ㅗ ㅏ → ㅘ). "
                     "⇧ on a letter without a ⇧ legend types as unshifted; ⇧ on a number or symbol key: its JIS character; ⇧⌥: its shifted JIS character.", keys=keys)

def k3s14():
    """Layout 14 as the engine reads it (docs/layouts/14-3set-jis.json): ⇧ is the key's JIS character."""
    d = json.load(open(os.path.join(DOCS, "14-3set-jis.json")))
    keys = {}
    for code in list(JIS) + ["Key"+c for c in LETTERS]:
        v = d["keys"].get(code, {"base": [], "shift": []})
        base = cell_of(v["base"])
        shift = cell_of(v["shift"])
        if code.startswith("Key"):
            shift = None                      # ⇧ letter: the capital, as on JIS
        keys[code] = {"base": base, "shift": shift}
    return dict(id=14, name="JIS 3-set", display="Korean (3-set JIS)", kind="henkan",
                note="세벌식 JIS (K3S-J), deterministic: initials on the right, vowels in the middle, finals on the left and the number row. "
                     "No jamo on ⇧: ⇧ types the key's JIS character. The same initial twice is its tense form (ㄱ ㄱ → ㄲ), so is ㄱ or ㅅ twice as a final; "
                     "vowels join (ㅗ ㅏ → ㅘ), finals cluster (ㄹ ㄱ → ㄺ).", keys=keys)

def search(id_, name, note):
    """Layouts 150 and 151: the keys spell a search."""
    out = lay(id_, "JIS " + name, name, "henkan", qwerty(), note=note)
    for code in ["Key"+c for c in LETTERS]:
        out["keys"][code]["base"]["c"] = "l"
        out["keys"][code]["shift"] = None
    return out

def roman18():
    """Layout 18: Korean romanization (docs/layouts/18-korean-roman.md)."""
    t = {}
    for c in LETTERS:
        t["Key"+c] = (c.lower(), c.lower())
    for code, v in JIS.items():
        t[code] = (v[0], v[1] if len(v) > 1 else "")
    t["Minus"] = ("-", "=")
    out = lay(18, "JIS Korean Roman", "Korean (Roman)", "henkan", t,
              note="As built (v0.1). Type Korean as it's romanized: annyeonghaseyo 안녕하세요, hanguk 한국, eo ㅓ, eu ㅡ, ae ㅐ, ng ㅇ. "
                   "- or ' splits syllables (hang-uk 항욱). The default reading shows; other readings that spell known words are offered "
                   "(k ㅋ/ㄱ, o ㅗ/ㅓ, u ㅜ/ㅡ, final t ㅅ/ㄷ/ㅆ…, m ㅂ: gamsahamnida → 감사합니다).")
    for code in ["Key"+c for c in LETTERS]:
        out["keys"][code]["base"]["c"] = "l"
        out["keys"][code]["shift"] = None
    out["keys"]["Minus"]["base"] = {"t": "-", "c": "x"}
    return out

ROMAN15_SPELLED = {}

def roman15():
    """Layout 15 as the engine types it (engine/src/bada_roman.rs, docs/layouts/15-bada-roman.json)."""
    d = json.load(open(os.path.join(DOCS, "15-bada-roman.json")))
    keys = {}
    for code in list(JIS) + ["Key"+c for c in LETTERS]:
        v = d["keys"].get(code, {"base": [], "shift": []})
        base = cell_of(v["base"])
        if base and code.startswith("Key"):
            base["c"] = "i" if base["t"] == "ㅇ" else "l"
        shift = None if code.startswith("Key") else cell_of(v["shift"])   # ⇧ letter: the capital
        if code in ROMAN15_SPELLED:                                       # the vowel keys show their romanization
            base = {"t": ROMAN15_SPELLED[code], "c": "v"}
        if code == "Semicolon":                                           # a syllable break: a dotted square, a break of no width
            base = {"t": "⬚", "c": "x", "s": 14}
        if base and not base["t"].isascii() and "s" not in base:              # Korean legends at the latin letters' size
            base["s"] = 19
        keys[code] = {"base": base, "shift": shift}
    return dict(id=15, name="bada roman", display="Korean (bada roman)", kind="henkan",
                no_jis=True,                               # no JIS keycap legends: the cells say what each key types
                note="Korean typed as it's romanized, as in Korean Roman (18), with keys of its own: q ch, x an initial ㅇ, "
                     "; a syllable break, - after e ㅐ / ㅒ (e- 애). "
                     "Number and symbol keys type their JIS characters (¥ ₩; ⇧0 +, ⇧[ 「, ⇧] 」).", keys=keys)

def kana19():
    """Layout 19 as the engine types it (engine/src/kana_korean.rs, docs/layouts/19-kana-korean.md)."""
    d = json.load(open(os.path.join(DOCS, "19-kana-korean.json")))
    keys = {}
    for code in list(JIS) + ["Key"+c for c in LETTERS]:
        v = d["keys"].get(code, {"base": [], "shift": []})
        base = cell_of(v["base"])
        shift_tokens = [dict(t, t="っ") if t["t"] == "NextJamoTenseIncrement" else t for t in v["shift"]]
        shift = cell_of(shift_tokens) or base
        if shift and shift["t"] == "っ":
            shift = {"t": "っ", "c": "k"}
        keys[code] = {"base": base, "shift": shift}
    return dict(id=19, name="JIS Kana Korean", display="Korean (Kana)", kind="henkan",
                note="As built (v0.1). Each kana key types its Korean approximation. ⇧3–⇧0 (small kana) change the previous vowel: き⇧ゃ → 캬. "
                     "゛ voices the previous initial (카゛ → 가), ゜ makes ㅎ ㅍ. ⇧ん ㅇ final, ⇧む ㅁ final, ⇧つ (っ) makes the next initial tense (⇧つ⇧ぁ 카; ん⇧ゃ 냐/야), ⇧の 의, ー after ㅔ/ㅖ makes ㅐ/ㅒ. "
                     "Shown: the table's reading; offered too: う-row ㅡ or a final (ク ㄱ, ム ㅁ, ル ㄹ, プ ㅂ), お-row ㅓ, ん ㅇ, っ as ㅅ (はんぐく → 한국).", keys=keys)

def romaji21():
    """Layout 21: romaji (docs/layouts/21-romaji.md). Letters compose; symbol keys give Japanese punctuation."""
    t = {}
    for c in LETTERS:
        t["Key"+c] = (c.lower(), "Ａ" if c == "A" else chr(0xFF21 + ord(c) - 65))
    jp = {"Minus": "ー", "Comma": "、", "Period": "。", "Slash": "・", "BracketRight": "「", "Backslash": "」"}
    for code, v in JIS.items():
        base = jp.get(code) or chr(ord(v[0]) + 0xFEE0) if "!" <= v[0] <= "~" else (jp.get(code) or v[0])
        sh = v[1] if len(v) > 1 else ""
        t[code] = (base, chr(ord(sh) + 0xFEE0) if sh and "!" <= sh <= "~" else sh)
    out = lay(21, "JIS Romaji", "Japanese (Romaji)", "henkan", t,
              note="As built: romaji with kanji conversion. Letters spell romaji as Japanese IMEs do (ka か, shi し, tsu つ, gakkou がっこう, nn ん). "
                   "⇧ + letter: the capital, full-width. - ー , 、 . 。 / ・ [ 「 ] 」; digits and symbols full-width. ⇧Tab: −1 hiragana, −2 katakana (script forms, off by default).")
    for code in ["Key"+c for c in LETTERS]:
        out["keys"][code]["base"]["c"] = "l"
    return out

if __name__ == "__main__":
    layouts = [
        bada13(),
        k3s14(),
        roman15(),
        roman18(),
        kana19(),
        None,
        romaji21(),
        lay(0, "JIS QWERTY", "QWERTY", "muhenkan", qwerty()),
        lay(10, "JIS KTS", "Korean (2-set)", "henkan", kts()),
        lay(11, "JIS Korean Legacy", "Korean (legacy-korean)", "henkan", legacy_korean(),
            note="As built: JIS Korean Mode A's Korean reading. Implied ㅇ; the same vowel twice keeps the final (마리 → 말이); ⇧ + vowel starts a syllable on ㅇ. ⇧Z is ㅋ here (F10 in JIS Korean)."),
        lay(12, "JIS Moonshot", "Korean (legacy-moonshot)", "henkan", legacy_moonshot(),
            note="As built: JIS Moonshot Mode A's Korean reading (no predictions). ⇧ consonant + ㅏ, ⇧ vowel on ㅇ; ㅙ ㅖ ㅒ ㅢ on − ; : , ."),
        search(150, "Symbols", "A search: letters and digits spell names (either case), Space adds a word (⇧Space: a space), a symbol offers its richer forms "
               "(. 。 …, ¥ the currencies). Names and tags (usd → $, money), emoji names and keywords (happy face → 😀)."),
        search(151, "Unicode", "A search of character names (snowman → ☃); a code gives its character (1f600, u+ac00). Space adds a word (⇧Space: a space)."),
        lay(20, "JIS Kana", "Japanese (Kana)", "henkan", kana(),
            note="As built: kana with kanji conversion. ゛ ゜ attach to the kana before them (か゛ が, は゜ ぱ). ⇧Tab: −1 hiragana, −2 katakana (script forms, off by default)."),
    ]
    kana20 = next(l for l in layouts if l and l["id"] == 20)
    layouts = [kana20 if l is None else l for l in layouts if l is not kana20]
    with open(os.path.join(HERE, "layouts.js"), "w") as f:
        f.write("// Generated by build_layouts.py. Do not edit.\nwindow.BADA_LAYOUTS = ")
        json.dump(layouts, f, ensure_ascii=False)
        f.write(";\n")
    print(len(layouts), "layouts")
