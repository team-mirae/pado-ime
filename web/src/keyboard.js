// The keyboard on screen: tools/render's drawing (geometry from tools/render/keyboards.js, copied in
// by web/build.mjs), with the legends of the layouts in use, read from docs/layouts/*.json (written
// by the engine: `cargo run --example layout_json`).

const NS = "http://www.w3.org/2000/svg";
const PAD = 14, G = 3, R = 7;
const el = (tag, attrs, text) => {
  const e = document.createElementNS(NS, tag);
  for (const k in attrs) e.setAttribute(k, attrs[k]);
  if (text != null) e.textContent = text;
  return e;
};

const INITIALS = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ", VOWELS = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ";
const syllable = (i, v) => String.fromCharCode(0xAC00 + (INITIALS.indexOf(i) * 21 + VOWELS.indexOf(v)) * 28);
/** What a token shows: role "a" is its consonant and ㅏ (가), role "n" its vowel on ㅇ (아), as the
 *  Keyboard Viewer shows them (mac/tools/make-keylayout.swift). */
const shown = e => e.r === "a" && INITIALS.includes(e.t) ? syllable(e.t, "ㅏ") : e.r === "n" && VOWELS.includes(e.t) ? syllable("ㅇ", e.t) : e.t;

/** A legend cell list ([{t, r}]) as {t, c}: several entries are one text, "ㅃ/ㅄ". */
function cell(list) {
  if (!list || !list.length) return null;
  return { t: list.map(shown).join("/"), c: list[0].r || "x" };
}

// Combination keycaps: each layout takes a corner by script, the next free one if it's taken.
const SLOT_PREF = id => (id >= 20 && id < 30 && id !== 21) ? ["BR", "BL", "TL"]
  : (id < 10 || id === 18 || id === 21 || id >= 100) ? ["TL", "BR", "BL"] : ["BL", "BR", "TL"];
const norm = t => t.normalize("NFKC").toLowerCase();

function rect(x, y, w, h) {
  const r = Math.min(R, w / 2, h / 2);
  return `M${x + r},${y}H${x + w - r}Q${x + w},${y} ${x + w},${y + r}V${y + h - r}Q${x + w},${y + h} ${x + w - r},${y + h}H${x + r}Q${x},${y + h} ${x},${y + h - r}V${y + r}Q${x},${y} ${x + r},${y}Z`;
}
function enter(x, y, U) { // 1.5u on top, 1.25u below
  const x0 = x + G, x1 = x + 0.25 * U + G, x2 = x + 1.5 * U - G, y0 = y + G, ym = y + U - G, y2 = y + 2 * U - G;
  return `M${x0 + R},${y0}H${x2 - R}Q${x2},${y0} ${x2},${y0 + R}V${y2 - R}Q${x2},${y2} ${x2 - R},${y2}H${x1 + R}Q${x1},${y2} ${x1},${y2 - R}V${ym}H${x0 + R}Q${x0},${ym} ${x0},${ym - R}V${y0 + R}Q${x0},${y0} ${x0 + R},${y0}Z`;
}

/**
 * Draws `kbName` ("hhkb" | "apple") with `layers` (layout JSONs, in candidate order) into `host`.
 * Returns code → key group. `active`: the language key of the mode ("Lang1" | "Lang2").
 */
export function drawKeyboard(host, { kbName, layers, jis, active }) {
  const kb = window.BADA_KEYBOARDS[kbName] || window.BADA_KEYBOARDS.hhkb;
  const multi = layers.length > 1;
  const U = multi ? 80 : 60;
  const W = kb.width * U + PAD * 2, H = kb.height * U + PAD * 2;
  const svg = el("svg", { viewBox: `0 0 ${W} ${H}`, xmlns: NS, role: "img",
    "aria-label": `${kb.name}: ${layers.map(l => l.name || l.id).join(" + ") || "no layout"}` });
  svg.append(el("rect", { x: 0.5, y: 0.5, width: W - 1, height: H - 1, rx: 14, fill: "var(--case)", stroke: "var(--case-edge)" }));
  const used = new Set();
  const slots = layers.map(l => { const s = SLOT_PREF(l.id).find(x => !used.has(x)) || "TL"; used.add(s); return s; });
  const byCode = {};
  for (const [code, kx, ky, kw, kh, label, shape] of kb.keys) {
    const x = PAD + kx * U, y = PAD + ky * U, w = kw * U, h = kh * U;
    const typing = code && layers.some(l => l.keys[code]);
    const g = el("g", { class: "key" + (typing ? "" : " mod") + (code === "Lang1" || code === "Lang2" ? " lang" : "") + (code === active ? " active" : "") });
    if (code) g.dataset.code = code;
    const d = shape === "ENTER" ? enter(x, y, U) : rect(x + G, y + G, w - 2 * G, h - 2 * G);
    g.append(el("path", { d, class: "cap" }));
    const text = (t, lx, ly, size, c, side, anchor = "start") => {
      const e = el("text", { x: lx, y: ly, "font-size": size, "text-anchor": anchor, class: `lg ${side} c-${c}` }, t);
      if (c === "f") e.setAttribute("text-decoration", "underline");
      g.append(e);
    };
    if (typing && multi) {
      const shown = new Set();
      const say = (t, lx, ly, size, c, side, anchor) => { text(t, lx, ly, size, c, side, anchor); t.split(/[\/ ]/).forEach(p => shown.add(norm(p))); };
      const fresh = c => c && c.t && !shown.has(norm(c.t));
      layers.forEach((lay, i) => {
        const k = lay.keys[code]; if (!k) return;
        let base = cell(k.base), shift = cell(k.shift);
        if (!fresh(base)) base = null;
        if (!fresh(shift)) shift = null;
        if (base && shift && norm(base.t) === norm(shift.t)) shift = null;
        const slot = slots[i];
        if (slot === "TL") {
          if (code.startsWith("Key") && base && /^[a-z]$/i.test(base.t)) { say(base.t.toUpperCase(), x + 11, y + 26, 18, base.c, "base"); }
          else {
            if (shift) say(shift.t, x + 11, y + 23, 13, shift.c, "shift");
            if (base) say(base.t, x + 11, y + 41, 13, base.c, "base");
          }
        } else if (slot === "BL") {
          if (base) {
            const [first, ...rest] = base.t.split("/");
            say(first, x + 11, y + h - 13, first.length > 2 ? 15 : 23, base.c, "base");
            if (rest.length) say(rest.join(" "), x + 38, y + h - 27, 12, base.c, "base");
          }
          if (shift && !["x", "l"].includes(shift.c) && !(base && base.t.includes("/"))) say(shift.t, x + 38, y + h - 27, 12, shift.c, "shift");
        } else {
          if (base) say(base.t, x + w - 11, y + h - 13, base.t.length > 2 ? 12 : 18, base.c, "base", "end");
          if (shift) say(shift.t, x + w - 11, y + h - 36, 12, shift.c, "shift", "end");
        }
      });
    } else if (typing) {
      const lay = layers[0], k = lay.keys[code];
      const base = cell(k.base), shift = cell(k.shift);
      const letter = code.startsWith("Key") && base && /^[a-z]$/.test(base.t) && shift && shift.t === base.t.toUpperCase();
      if (letter) text(shift.t, x + 12, y + 30, 22, shift.c, "base");
      else {
        if (base) {
          const [first, ...rest] = base.t.split("/");
          text(first, x + 12, y + h - 13, first.length > 2 ? 15 : 24, base.c, "base");
          if (rest.length) text(rest.join(" "), x + 12, y + h - 38, 11, base.c, "base");
        }
        if (shift && (!base || shift.t !== base.t)) text(shift.t, x + 12, y + 24, shift.t.length > 2 ? 11 : 15, shift.c, "shift");
      }
      if (jis && lay.id !== 0 && window.BADA_JIS[code]) {
        const [b, s] = window.BADA_JIS[code];
        const isLetter = code.startsWith("Key");
        g.append(el("text", { x: x + w - 10, y: y + h - 11, "font-size": 11, "text-anchor": "end", class: "jis" }, isLetter ? s : b));
        if (!isLetter && s) g.append(el("text", { x: x + w - 10, y: y + 20, "font-size": 11, "text-anchor": "end", class: "jis" }, s));
      }
    } else if (label != null) {
      const lines = label.split("/");
      const size = (h < U ? 12 : lines.length > 1 || label.length > 4 ? 11 : 14) * (multi ? 1.2 : 1);
      const cy = shape === "ENTER" ? y + U / 2 : y + h / 2;
      const cx = shape === "ENTER" ? x + 0.75 * U : x + w / 2;
      lines.forEach((ln, i) => g.append(el("text", { x: cx, y: cy + 4 + (i - (lines.length - 1) / 2) * 13, "font-size": size, "text-anchor": "middle", class: "modl" }, ln)));
    }
    svg.append(g);
    if (code) byCode[code] = g;
  }
  host.replaceChildren(svg);
  return byCode;
}
