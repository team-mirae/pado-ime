"""Render pado's icon (docs/images/icon/pado-icon.svg) to PNGs beside it and to windows/pado/pado.ico.

    python windows/tools/make-icon.py [--browser PATH]

Uses headless Edge (or Chrome) and Pillow (`pip install pillow`). The .ico is the one Windows shows in
its language list and the settings window's; the indicator shows text (ime.json's labels).
Adapted from tools/icon/export.py (bada's).
"""
import argparse, os, subprocess, tempfile, pathlib

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SVG = ROOT / "docs/images/icon/pado-icon.svg"
BROWSERS = [
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--browser", default=next((b for b in BROWSERS if os.path.exists(b)), None))
    a = ap.parse_args()
    if not a.browser:
        raise SystemExit("no Edge or Chrome found: pass --browser")
    from PIL import Image
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        page = tmp / "icon.html"
        page.write_text(f'<html><body style="margin:0"><img src="{SVG.as_uri()}" width="1024" height="1024"></body></html>')
        raw = tmp / "raw.png"
        # headless viewports are shorter than their windows: shoot taller, then crop
        subprocess.run([a.browser, "--headless=new", f"--user-data-dir={tmp / 'profile'}", "--hide-scrollbars",
                        "--allow-file-access-from-files", "--default-background-color=00000000",
                        "--force-device-scale-factor=1", "--window-size=1024,1200", f"--screenshot={raw}",
                        page.as_uri()], check=True, capture_output=True)
        icon = Image.open(raw).convert("RGBA").crop((0, 0, 1024, 1024))
    for size in (1024, 512):
        out = SVG.with_name(f"pado-icon-{size}.png")
        (icon if size == 1024 else icon.resize((size, size), Image.LANCZOS)).save(out, optimize=True)
        print(out, out.stat().st_size, "bytes")
    ico = ROOT / "windows/pado/pado.ico"
    icon.resize((256, 256), Image.LANCZOS).save(ico, sizes=[(s, s) for s in (16, 20, 24, 32, 40, 48, 64, 128, 256)])
    print(ico, ico.stat().st_size, "bytes")


if __name__ == "__main__":
    main()
