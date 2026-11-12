// bada ime in the browser: the engine (WebAssembly) behind a text field, as the macOS shell runs it
// (mac/Sources/InputController.swift, LanguageKeyTap.swift): keys in, text and a menu out. All
// behaviour is the engine's; this file moves keys in, applies the output, and switches modes.

import { loadEngine } from "./load.js";
import * as S from "./settings.js";
import { drawKeyboard } from "./keyboard.js";
import { openSettings } from "./settings-ui.js";

const $ = id => document.getElementById(id);
const ta = $("text"), backdrop = $("backdrop"), menuEl = $("menu"), board = $("board");
const DICT_STORE = "bada-dictionary";
const MAC = /Mac/.test(navigator.platform || navigator.userAgent);
const MODIFIERS = new Set(["ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight", "CapsLock", "Fn", "FnLock"]);
const coarse = matchMedia("(pointer: coarse)").matches;

let engine = null, catalog = { layouts: [], actions: [] }, version = "";
const layouts = {};   // id → layout JSON (docs/layouts)
let settings = S.load(), web = S.loadWeb();
const state = {
  mode: "henkan", profile: S.MAC_KANA,
  mark: null,            // the marked text in the field: {start, len}
  menuRows: 0, out: null,
  latched: { shift: false, ctrl: false, alt: false, meta: false },
  caps: false,           // Caps Lock as bada ime holds it (spec §6.7)
  tap: null,             // a ⌘ pressed alone so far (⌘ taps as 英数 / かな)
  dictStatus: "No dictionary yet.",
};
let keys = {};   // code → key group on screen

// ── Theme ──────────────────────────────────────────────────────────────────────────────────────
const dark = matchMedia("(prefers-color-scheme: dark)");
function applyTheme() {
  document.documentElement.dataset.theme = web.theme === "auto" ? (dark.matches ? "dark" : "light") : web.theme;
}
dark.addEventListener("change", applyTheme);
applyTheme();

// ── Loading ────────────────────────────────────────────────────────────────────────────────────

async function start() {
  const loaded = await loadEngine((got, total, phase) => {
    $("progress").style.width = `${Math.min(100, (100 * got) / total).toFixed(1)}%`;
    $("progressText").textContent = phase === "starting" ? "Starting the engine…" : `${(got / 1e6).toFixed(1)} of ${(total / 1e6).toFixed(1)} MB`;
  });
  engine = loaded.engine;
  version = loaded.manifest.version || "";
  Object.assign(layouts, loaded.layouts);
  catalog = engine.catalog();
  loadDictionary(localStorage.getItem(DICT_STORE) || "");
  applySettings();
  engine.setMode(state.mode);
  engine.setProfile(state.profile);
  $("loading").hidden = true;
  ta.disabled = false;
  ta.placeholder = "Type here. かな: henkan mode (Korean, Japanese…), 英数: muhenkan mode (QWERTY).";
  if (coarse) ta.inputMode = "none";   // the keyboard below is the keyboard
  ta.focus();
  refresh();
}

// ── Settings ───────────────────────────────────────────────────────────────────────────────────

function applySettings() {
  engine.applySettings(S.engineJSON(settings));
  if (!settings.use_profiles && state.profile !== S.MAC_KANA) {
    state.profile = S.MAC_KANA;
    apply(engine.setProfile(state.profile));
  }
}

function loadDictionary(text) {
  const summary = engine.loadDictionary(text);
  if (!text) state.dictStatus = "No dictionary yet.";
  else if (summary.ok) state.dictStatus = `Loaded: ${summary.positive ?? 0} positive, ${summary.negative ?? 0} negative.`;
  else state.dictStatus = `The dictionary wasn't loaded: ${summary.error ?? "unknown error"}`;
  return summary;
}

const api = {
  get settings() { return settings; },
  set settings(next) {
    settings = S.normalize(next);
    S.save(settings);
    applySettings();
    refresh();
  },
  get web() { return web; },
  set web(next) {
    web = next;
    S.saveWeb(web);
    applyTheme();
    refresh();
  },
  get catalog() { return catalog; },
  get version() { return version; },
  get dictionary() { return localStorage.getItem(DICT_STORE) || ""; },
  set dictionary(text) {
    try { text ? localStorage.setItem(DICT_STORE, text) : localStorage.removeItem(DICT_STORE); } catch {}
    loadDictionary(text);
  },
  get dictStatus() { return state.dictStatus; },
};

$("settingsBtn").onclick = () => engine && openSettings(api);

// ── The text field ─────────────────────────────────────────────────────────────────────────────

function replace(start, end, text) {
  ta.setRangeText(text, start, end, "end");
}

/** Applies an engine output to the field (InputController.apply). */
function apply(out, depth = 0) {
  if (!out) return;
  const m = state.mark;
  if (out.commit) {
    const [s, e] = !out.replace_selection && m ? [m.start, m.start + m.len] : [ta.selectionStart, ta.selectionEnd];
    replace(s, e, out.commit);
    state.mark = null;
  }
  if (out.preedit) {
    const [s, e] = state.mark ? [state.mark.start, state.mark.start + state.mark.len] : [ta.selectionStart, ta.selectionEnd];
    replace(s, e, out.preedit);
    state.mark = { start: s, len: out.preedit.length };
  } else if (state.mark) {
    replace(state.mark.start, state.mark.start + state.mark.len, "");
    state.mark = null;
  }
  state.out = out;
  state.menuRows = out.menu.length;
  // The engine needs the selection (hanja, 半角/全角 with nothing staged).
  if (out.need_selection && depth === 0) {
    const text = !state.mark && ta.selectionEnd > ta.selectionStart ? ta.value.slice(ta.selectionStart, ta.selectionEnd) : "";
    apply(engine.hostSelection(text), 1);
    return;
  }
  drawField();
}

function insert(text) {
  replace(ta.selectionStart, ta.selectionEnd, text);
  drawField();
}

const esc = t => t.replace(/[&<>]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]);

/** The backdrop (the marked text's underline) and the menu under it. */
function drawField() {
  const v = ta.value, m = state.mark;
  const at = m ? m.start : ta.selectionStart;
  const marked = m ? `<span class="mark">${esc(v.slice(m.start, m.start + m.len))}</span>` : "";
  backdrop.innerHTML = `${esc(v.slice(0, at))}<span id="anchor"></span>${marked}${esc(v.slice(m ? m.start + m.len : at))}\n`;
  backdrop.scrollTop = ta.scrollTop;
  drawMenu();
}

function drawMenu() {
  const out = state.out;
  if (!out || !out.menu.length) { menuEl.hidden = true; return; }
  menuEl.className = "menu" + (out.horizontal ? " horizontal" : "");
  menuEl.replaceChildren(...out.menu.map((item, i) => {
    const sel = i === out.selected;
    if (out.horizontal) {
      const c = document.createElement("div");
      c.className = "cell" + (sel ? " sel" : "");
      c.innerHTML = `<span>${esc(item)}</span>${i < 10 ? `<small>${(i + 1) % 10}</small>` : "<small></small>"}`;
      return c;
    }
    const row = document.createElement("div");
    row.className = "row" + (sel ? " sel" : "");
    const text = document.createElement("span");
    text.textContent = item;
    if (out.japanese[i]) text.className = "ja";
    row.append(text);
    if (out.notes[i]) {
      const note = document.createElement("span");
      note.className = "note" + (/[぀-￿]/.test(out.notes[i]) ? " ja" : "");
      note.textContent = out.notes[i];
      row.append(note);
    }
    return row;
  }));
  menuEl.hidden = false;
  // Under the start of the marked text, as on the Mac.
  const anchor = $("anchor"), editor = $("editor").getBoundingClientRect();
  const a = (anchor.nextElementSibling?.getClientRects()[0]) || anchor.getBoundingClientRect();
  const width = menuEl.offsetWidth;
  const left = Math.max(4, Math.min(a.left - editor.left - 12, editor.width - width - 4));
  menuEl.style.left = `${left}px`;
  menuEl.style.top = `${a.bottom - editor.top + 6}px`;
  const selRow = menuEl.children[out.selected];
  if (selRow && !out.horizontal) {
    const top = selRow.offsetTop, h = selRow.offsetHeight;
    if (top < menuEl.scrollTop) menuEl.scrollTop = top - 5;
    else if (top + h > menuEl.scrollTop + menuEl.clientHeight) menuEl.scrollTop = top + h - menuEl.clientHeight + 5;
  }
}

ta.addEventListener("scroll", () => { backdrop.scrollTop = ta.scrollTop; drawMenu(); });
new ResizeObserver(() => drawField()).observe(ta);
// A click in the text or leaving the field commits what's staged (commitComposition).
ta.addEventListener("mousedown", () => { if (state.mark) apply(engine.flush()); });
ta.addEventListener("blur", () => { if (engine && state.mark) apply(engine.flush()); });
ta.addEventListener("input", () => {
  if (state.mark) { state.mark = null; engine.flush(); }   // the text changed under the marked text
  state.out = null;
  drawField();
});
ta.addEventListener("select", () => drawField());

// ── Modes and language keys ────────────────────────────────────────────────────────────────────

/** To `mode`, in place: commits what's staged (InputController.switchMode). */
function switchMode(mode) {
  apply(engine.setMode(mode));
  engine.setProfile(state.profile);
  state.mode = mode;
  // Leaving muhenkan forgets Caps Lock, unless it's respected (spec §6.7).
  if (mode !== "muhenkan" && settings.caps_lock !== "respect") state.caps = false;
  drawCaps();
  refresh();
}

/** Caps Lock pressed (spec §6.7), as CapsLock.key in mac/Sources/CapsLock.swift. */
function capsLock(mods) {
  const modified = mods.shift || mods.ctrl || mods.alt || mods.meta;
  if (settings.caps_lock === "toggle" && !modified) {
    state.caps = false;
    switchMode(state.mode === "henkan" ? "muhenkan" : "henkan");
    blinkCaps();
    return;
  }
  if (settings.caps_lock === "respect") state.caps = !state.caps;
  else state.caps = state.mode === "muhenkan" ? !state.caps : false;
  drawCaps();
}

/** The Caps Lock light: the key below, lit while Caps Lock is on. */
function drawCaps(lit = state.caps) {
  keys.CapsLock?.classList.toggle("latched", lit);
}

function blinkCaps() {   // a mode switch by Caps Lock always leaves Caps Lock off: a flash on
  drawCaps(true);
  setTimeout(() => drawCaps(), 150);
}

function switchProfile(name) {
  state.profile = name;
  apply(engine.setProfile(name));
  refresh();
}

/** かな or a profile key in henkan mode (spec §6.6). */
function languageKey(key, shift) {
  const a = S.profileKeyAction(settings, key, state.profile, shift, state.menuRows > 1);
  if ("step" in a) apply(engine.key("Lang1", { shift: a.step }));
  else if (a.profile) switchProfile(a.profile);
  else switchMode("muhenkan");
}

function profileKeyFor(code) {
  if (!settings.use_profiles) return null;
  const mac = S.macCode(code);
  if (mac == null || mac === 104 || mac === 102) return null;
  for (const [name, recorded] of Object.entries(settings.profile_keys)) if (recorded === mac) return name;
  return null;
}

$("muhenkanBtn").onclick = () => { if (engine) { press("Lang2", {}); ta.focus(); } };
$("henkanBtn").onclick = () => { if (engine) { press("Lang1", {}); ta.focus(); } };

// ── Keys ───────────────────────────────────────────────────────────────────────────────────────

/** What layout 0 (QWERTY, the muhenkan layout) types for a key passed on: macOS's ABC, JIS legends. */
function passChar(code, shift) {
  const k = (layouts[settings.default_muhenkan] || layouts[0])?.keys[code];
  const cell = k && (shift ? k.shift : k.base);
  return cell && cell.length ? cell[0].t : null;
}

/**
 * One key press. Returns true when it was dealt with here (the browser's own handling is
 * prevented); false: the browser does what it does with the key (Backspace, arrows, shortcuts).
 * `synthetic`: a click on the keyboard below, which has no browser handling to fall back on.
 */
function press(code, mods, { repeat = false, key = "", synthetic = false } = {}) {
  mods = { shift: !!mods.shift, ctrl: !!mods.ctrl, alt: !!mods.alt, meta: !!mods.meta };
  const pk = profileKeyFor(code);
  const eisu = code === "Lang2" || key === "Eisu" || key === "Alphanumeric";
  const kana = !pk && (code === "Lang1" || code === "KanaMode" || key === "KanaMode");
  if (eisu) {
    if (!repeat) switchMode("muhenkan");
    return true;
  }
  if (kana || (pk && !mods.ctrl && !mods.alt && !mods.meta)) {
    if (!repeat) {
      if (kana && mods.ctrl && mods.alt && mods.shift) switchMode(state.mode === "henkan" ? "muhenkan" : "henkan");   // HYPER / MEH
      else if (kana && mods.ctrl && !mods.alt && !mods.meta && !mods.shift) { if (state.mode === "henkan") apply(engine.key("Lang1", { ctrl: true })); }   // ⌃かな: fix what shows
      else if (state.mode === "henkan") languageKey(pk ?? S.MAC_KANA, mods.shift);
      else {
        state.profile = settings.use_profiles ? pk ?? S.MAC_KANA : S.MAC_KANA;
        switchMode("henkan");
      }
    }
    return true;
  }
  // Caps Lock's UPPER_SNAKE rule (spec §6.2): Space types _, ⌥Space a space.
  if (state.caps && state.mode === "muhenkan" && code === "Space" && !mods.ctrl && !mods.meta && settings.caps_rule === "uppersnake") {
    apply(engine.flush());
    insert(mods.alt ? " " : "_");
    return true;
  }
  const out = engine.key(code, mods);
  apply(out);
  if (out.consumed) return true;
  return passOn(code, mods, synthetic);
}

/** A key the engine passed on: what the app (a text field, through macOS's ABC layout) does. */
function passOn(code, mods, synthetic) {
  if (mods.meta || mods.ctrl) return !synthetic ? false : true;
  let ch = passChar(code, mods.shift);
  if (ch && state.caps) ch = ch.toUpperCase();   // Caps Lock: capitals, ⇧ or not (as macOS has it)
  if (ch && (!mods.alt || synthetic)) { insert(ch); return true; }
  switch (code) {
    case "Space": insert(" "); return true;
    case "Tab": insert("\t"); return true;
    case "Enter": insert("\n"); return true;
    case "Escape": return true;
  }
  if (!synthetic) return false;
  const s = ta.selectionStart, e = ta.selectionEnd;
  switch (code) {
    case "Backspace":
      if (s !== e) replace(s, e, "");
      else if (s > 0) { const n = /[\uDC00-\uDFFF]/.test(ta.value[s - 1]) && s > 1 ? 2 : 1; replace(s - n, s, ""); }
      break;
    case "ArrowLeft": ta.setSelectionRange(Math.max(0, s - (s === e ? 1 : 0)), Math.max(0, s - (s === e ? 1 : 0))); break;
    case "ArrowRight": ta.setSelectionRange(e + (s === e ? 1 : 0), e + (s === e ? 1 : 0)); break;
    case "ArrowUp": ta.setSelectionRange(0, 0); break;
    case "ArrowDown": ta.setSelectionRange(ta.value.length, ta.value.length); break;
  }
  drawField();
  return true;
}

function keyDown(e) {
  if ($("settings").open || !engine) return;
  const target = e.target;
  if (target !== ta && target !== document.body && !board.contains(target)) return;
  keys[e.code]?.classList.add("on");
  if (e.code.startsWith("Shift")) board.classList.add("shift");
  if (e.isComposing || e.keyCode === 229) {
    warn("Your Mac's own input method is composing: switch it to ABC (or another plain keyboard) so bada ime gets the keys.");
    return;
  }
  if (target !== ta) ta.focus();
  if (e.code === "MetaLeft" || e.code === "MetaRight") { state.tap = e.repeat ? null : e.code; return; }
  if (e.code === "CapsLock") {
    if (!e.repeat) capsLock({ shift: e.shiftKey, ctrl: e.ctrlKey, alt: e.altKey, meta: e.metaKey });
    e.preventDefault();
    return;
  }
  if (MODIFIERS.has(e.code)) return;
  state.tap = null;
  const code = e.code === "NumpadEnter" ? "Enter" : e.code;
  const mods = { shift: e.shiftKey, ctrl: e.ctrlKey, alt: e.altKey, meta: e.metaKey };
  if (press(code, mods, { repeat: e.repeat, key: e.key })) e.preventDefault();
}

function keyUp(e) {
  keys[e.code]?.classList.remove("on");
  // A Mac browser reports Caps Lock going off as a keyup alone: that's a press too.
  if (e.code === "CapsLock" && MAC && engine && !$("settings").open) capsLock({ shift: e.shiftKey, ctrl: e.ctrlKey, alt: e.altKey, meta: e.metaKey });
  if (e.code.startsWith("Shift") && !state.latched.shift) board.classList.remove("shift");
  // ⌘ alone, as the ⌘英かな convention has it: left ⌘ 英数, right ⌘ かな.
  if (web.cmdTaps && state.tap === e.code && engine && !$("settings").open) {
    state.tap = null;
    press(e.code === "MetaLeft" ? "Lang2" : "Lang1", { shift: e.shiftKey });
  }
  state.tap = null;
}

addEventListener("keydown", keyDown);
addEventListener("keyup", keyUp);
addEventListener("blur", () => { Object.values(keys).forEach(g => g.classList.remove("on")); board.classList.remove("shift"); state.tap = null; });
addEventListener("mousedown", () => { state.tap = null; });

function warn(text) {
  const w = $("warning");
  w.textContent = text;
  w.hidden = false;
  clearTimeout(warn.timer);
  warn.timer = setTimeout(() => { w.hidden = true; }, 8000);
}

// Clicks on the keyboard below: a key press; modifiers latch for the next key.
const LATCH = { ShiftLeft: "shift", ShiftRight: "shift", ControlLeft: "ctrl", AltLeft: "alt", AltRight: "alt", MetaLeft: "meta", MetaRight: "meta" };
board.addEventListener("pointerdown", e => {
  const g = e.target.closest(".key");
  e.preventDefault();   // keep the focus in the text
  if (!g || !g.dataset.code || !engine) return;
  const code = g.dataset.code;
  if (LATCH[code]) {
    state.latched[LATCH[code]] = !state.latched[LATCH[code]];
    drawLatched();
    return;
  }
  if (code === "CapsLock") {
    capsLock(state.latched);
    state.latched = { shift: false, ctrl: false, alt: false, meta: false };
    drawLatched();
    return;
  }
  if (document.activeElement !== ta && !coarse) ta.focus();
  g.classList.add("on");
  setTimeout(() => g.classList.remove("on"), 120);
  press(code, state.latched, { synthetic: true });
  state.latched = { shift: false, ctrl: false, alt: false, meta: false };
  drawLatched();
});

function drawLatched() {
  for (const [code, mod] of Object.entries(LATCH)) keys[code]?.classList.toggle("latched", state.latched[mod]);
  board.classList.toggle("shift", state.latched.shift);
  drawCaps();
}

// ── What's shown around the field ──────────────────────────────────────────────────────────────

const layoutName = id => catalog.layouts.find(l => l.id === id)?.name ?? `Layout ${id}`;

function refresh() {
  const henkan = state.mode === "henkan";
  $("muhenkanBtn").classList.toggle("on", !henkan);
  $("henkanBtn").classList.toggle("on", henkan);
  const ids = henkan ? S.candidates(settings, state.profile) : [settings.default_muhenkan];
  const layers = ids.map(id => layouts[id]).filter(Boolean);
  keys = drawKeyboard(board, { kbName: web.keyboard, layers, jis: web.jis, active: henkan ? "Lang1" : "Lang2" });
  drawLatched();
  const status = $("status");
  status.innerHTML = "";
  const item = (label, value) => { const s = document.createElement("span"); s.innerHTML = `${label} <b></b>`; s.querySelector("b").textContent = value; status.append(s); };
  item("Mode", henkan ? "Henkan" : "Muhenkan");
  if (henkan && settings.use_profiles) item("Profile", `${S.PROFILE_KEY[state.profile]} (profile-${state.profile})`);
  item(henkan ? "Candidates" : "Layout", ids.length ? ids.map(layoutName).join(" + ") : "none (Settings → Layouts)");
  $("help").innerHTML = "<kbd>英数</kbd> muhenkan · <kbd>かな</kbd> henkan, again: next entry (<kbd>⇧</kbd> previous) · "
    + (settings.caps_lock === "toggle" ? "<kbd>Caps Lock</kbd> the other mode · " : "")
    + "<kbd>Tab</kbd> <kbd>↓</kbd> / <kbd>⇧Tab</kbd> <kbd>↑</kbd> move in the menu · <kbd>Space</kbd> <kbd>Enter</kbd> <kbd>Esc</kbd> commit"
    + (web.cmdTaps ? " · No 英数 / かな keys? Tap left <kbd>⌘</kbd> / right <kbd>⌘</kbd>, or click them below." : " · Click 英数 / かな below.");
  drawField();
}

start().catch(err => {
  $("progressText").textContent = `Couldn't load bada ime: ${err.message}`;
  console.error(err);
});
