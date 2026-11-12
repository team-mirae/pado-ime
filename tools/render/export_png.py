"""Render layout and combination PNGs for the docs with headless Chrome.

    python3 tools/render/export_png.py [--kb hhkb] [--chrome PATH] [--only NAME ...]

Writes docs/images/layouts/<name>-<theme>.png (light and dark, 2x; macOS: crops with sips) through index.html's capture
mode (?layers=…&kb=…&theme=…&capture). Run build_layouts.py first if the layouts changed.
"""
import argparse, json, os, subprocess, sys, tempfile, urllib.parse

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.normpath(os.path.join(HERE, "../../docs/images/layouts"))
CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"

# name → layout ids (one id: a layout on its own; several: enabled together in henkan mode)
IMAGES = {
    "00-qwerty": [0], "10-kts": [10], "11-korean-legacy": [11], "12-moonshot": [12], "13-bada": [13], "14-3set-jis": [14],
    "15-bada-roman": [15], "16-bada-plain": [16], "18-korean-roman": [18], "19-kana-korean": [19], "20-kana": [20], "21-romaji": [21],
    "qwerty-bada-plain-kana": [0, 16, 20], "qwerty-bada-kana": [0, 13, 20], "qwerty-bada-roman-kana": [0, 15, 20], "bada-romaji": [13, 21], "qwerty-romaji": [0, 21], "qwerty-kts": [0, 10],
}
SIZE = {"hhkb": 15, "apple": 14.5}   # board width in key units (keyboards.js)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--kb", default="hhkb", choices=SIZE)
    ap.add_argument("--chrome", default=CHROME)
    ap.add_argument("--scale", type=float, default=2)
    ap.add_argument("--only", nargs="*", help="image names to render (default: all)")
    a = ap.parse_args()
    os.makedirs(OUT, exist_ok=True)
    page = "file://" + urllib.parse.quote(os.path.join(HERE, "index.html"))
    with tempfile.TemporaryDirectory() as profile:
        for name, ids in IMAGES.items():
            if a.only and name not in a.only:
                continue
            u = 80 if len(ids) > 1 else 60
            w, h = round(SIZE[a.kb] * u + 28), 5 * u + 28 + 26
            for theme in ("light", "dark"):
                q = urllib.parse.urlencode({"kb": a.kb, "layers": ",".join(map(str, ids)), "theme": theme, "jis": 1, "capture": ""})
                out = os.path.join(OUT, f"{name}-{theme}.png")
                # headless Chrome's viewport is shorter than its window: shoot taller, then crop to the board
                subprocess.run([a.chrome, "--headless=new", f"--user-data-dir={profile}", "--hide-scrollbars",
                                "--default-background-color=00000000", f"--force-device-scale-factor={a.scale}",
                                f"--window-size={w + 1},{h + 200}", f"--screenshot={out}", f"{page}?{q}"],
                               check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                # the page draws the board 1px from the left: sips reads an offset of 0 0 as "centre"
                subprocess.run(["sips", "--cropOffset", "0", str(round(a.scale)), "-c", str(round(h * a.scale)), str(round(w * a.scale)), out],
                               check=True, stdout=subprocess.DEVNULL)
                print(os.path.relpath(out), flush=True)


if __name__ == "__main__":
    sys.exit(main())
