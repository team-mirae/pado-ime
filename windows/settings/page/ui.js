// pado's settings window: bada's settings dialog (web/src/settings-ui.js, itself the Mac's
// SettingsWindow.swift) on Windows. The same settings, rules and bada-settings/1 file, through
// web/src/settings.js unchanged; kept in the registry by pado-settings.exe (src/main.rs).
// Differences from the Mac: the profile keys are fixed (変換, カタカナ/ひらがな, an Apple keyboard's かな),
// Caps Lock is bada-windows' (windows/host/docs/spec.md §5), and a Windows page has the host's own
// settings. A change to web/src/settings-ui.js needs the same change here (CLAUDE.md).

import * as S from "./settings.js";

const $ = id => document.getElementById(id);
const PAGES = ["Layouts", "Profiles", "Menu", "F Keys", "Dictionary", "Backup", "Windows", "About"];
/** Every order of the three profiles, for the cycle order. */
const ORDERS = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]].map(o => o.map(i => S.PROFILES[i][0]));
/** The profiles' keys on Windows (windows/host/docs/spec.md §4.2). */
const WIN_KEY = { "mac-kana": "かな (Apple)", henkan: "変換", katakana: "カタカナ" };
const KEY_NOTE = {
  "mac-kana": "An Apple JIS keyboard's かな (Windows page). PC keyboards have none.",
  henkan: "変換: the home key on Windows. Without profiles, every key gives this one henkan.",
  katakana: "カタカナ/ひらがな.",
};
/** Caps Lock on Windows (windows/host/docs/spec.md §5; the Mac's §6.7). */
const CAPS_LOCK = [
  ["respect", "Windows' own Caps Lock (default on Windows)"],
  ["toggle", "Switch mode (⇧ + Caps Lock: Caps Lock, in muhenkan)"],
  ["transient", "Caps Lock in muhenkan only, off when you leave it"],
];

const ui = { tab: "Layouts", editing: S.MAC_KANA, fEditing: S.MAC_KANA, note: "" };
const api = { settings: S.defaults(), catalog: { layouts: [], actions: [] }, dictionary: "", dictStatus: "", windows: {} };
let base = location.pathname.replace(/\/$/, "");

function h(tag, props = {}, ...children) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k.startsWith("on")) e.addEventListener(k.slice(2), v);
    else if (k === "class") e.className = v;
    else if (v === true) e.setAttribute(k, "");
    else if (v !== false && v != null) { if (k in e && k !== "list") e[k] = v; else e.setAttribute(k, v); }
  }
  for (const c of children.flat()) if (c != null && c !== false) e.append(c);
  return e;
}
const heading = t => h("h3", {}, t);
const hint = t => h("p", { class: "hint" }, t);
const row = (...c) => h("div", { class: "row" }, ...c);
function check(label, checked, onchange, disabled = false) {
  return h("label", { class: "check" }, h("input", { type: "checkbox", checked, disabled, onchange: e => onchange(e.target.checked) }), h("span", {}, label));
}
function select(options, value, onchange, props = {}) {
  const s = h("select", { ...props, onchange: e => onchange(e.target.value) },
    ...options.map(([v, label]) => h("option", { value: v }, label)));
  s.value = value;
  return s;
}
function link(url, text) {
  return h("a", { href: url, onclick: e => { e.preventDefault(); call("POST", "/api/open", url); } }, text);
}

// ── The server ───────────────────────────────────────────────────────────────────────────────────

function status(text, error = false) {
  $("status").textContent = text;
  $("status").className = error ? "err" : "";
}

async function call(method, path, body) {
  try {
    const r = await fetch(base + path, { method, body: typeof body === "string" ? body : body == null ? undefined : JSON.stringify(body) });
    const v = await r.json();
    if (!r.ok) throw new Error(v.error || r.statusText);
    return v;
  } catch (e) {
    status(`Not saved: ${e.message}. Close this window and open Settings again.`, true);
    throw e;
  }
}

async function load() {
  const state = await call("GET", "/api/state");
  api.settings = S.normalize(state.settings);
  // Caps Lock lives with the host: its registry values, Windows' defaults.
  api.settings.caps_lock = state.windows.caps_lock;
  api.settings.caps_rule = state.windows.caps_rule;
  api.catalog = state.catalog;
  api.dictionary = state.dictionary;
  api.dictStatus = state.dictStatus;
  api.dictionaryPath = state.dictionaryPath;
  api.windows = state.windows;
  api.version = state.version;
  api.engineVersion = state.engineVersion;
}

let saving = Promise.resolve();
function save() {
  const settings = structuredClone(api.settings);
  saving = saving.then(() => call("PUT", "/api/settings", { settings, engine: S.engineJSON(settings) }))
    .then(() => status("Saved. Apps pick it up when you next click or switch into them."), () => {});
}

/** Changes the settings (a copy, `f` edits it), saves them and redraws the page. */
function change(f) {
  const s = structuredClone(api.settings);
  f(s);
  api.settings = s;
  save();
  draw();
}

function changeWindows(f) {
  const w = { ...api.windows };
  f(w);
  api.windows = w;
  call("PUT", "/api/windows", w).then(() => status("Saved."), () => {});
  draw();
}

// ── Pages ────────────────────────────────────────────────────────────────────────────────────────

const layoutName = id => api.catalog.layouts.find(l => l.id === id)?.name ?? `Layout ${id}`;
const weightText = w => String(w ?? 1);
const profileOptions = () => S.PROFILES.map(([name]) => [name, `${WIN_KEY[name]}  (profile-${name})`]);

function draw() {
  const s = api.settings;
  if (!s.use_profiles) { ui.editing = S.MAC_KANA; ui.fEditing = S.MAC_KANA; }
  $("tabs").replaceChildren(...PAGES.map(p => h("button", {
    role: "tab", "aria-selected": String(p === ui.tab), onclick: () => { ui.tab = p; ui.note = ""; draw(); },
  }, p)));
  const page = { Layouts: layoutsPage, Profiles: profilesPage, Menu: menuPage, "F Keys": fKeysPage,
    Dictionary: dictionaryPage, Backup: backupPage, Windows: windowsPage, About: aboutPage }[ui.tab](s);
  $("pages").replaceChildren(h("div", { class: "page", role: "tabpanel" }, ...page));
}

function layoutsPage(s) {
  const linked = S.link(s, ui.editing) != null;
  const list = S.own(s, ui.editing);
  return [
    heading("Henkan candidates"),
    hint("Every enabled layout reads the keys at once; the readings are ranked as you type."),
    row("Candidates of", select(profileOptions(), ui.editing, v => { ui.editing = v; draw(); }, { disabled: !s.use_profiles })),
    linked ? hint(`This profile uses ${WIN_KEY[S.link(s, ui.editing)]}'s candidates (Profiles tab).`) : null,
    hint("× weight: multiplies the score of everything that layout offers, in every profile (1: as is, 0–10)."),
    h("div", { class: "cands" }, ...api.catalog.layouts.map(l => h("div", { class: "cand" },
      h("label", {}, h("input", { type: "checkbox", checked: list.includes(l.id), disabled: linked, onchange: e => change(s => {
        // The list in catalog order, as the Mac's checkboxes give it.
        const on = new Set(S.own(s, ui.editing));
        e.target.checked ? on.add(l.id) : on.delete(l.id);
        const next = api.catalog.layouts.map(x => x.id).filter(id => on.has(id));
        if (ui.editing === S.MAC_KANA) s.enabled = next;
        else (s.profiles[ui.editing] ??= { link: null, enabled: [] }).enabled = next;
      }) }), `${l.name}  (${l.id})`),
      "×",
      h("input", { type: "number", min: 0, max: 10, step: 0.1, value: weightText(s.weights[String(l.id)]), "aria-label": `Weight for ${l.name}`,
        onchange: e => change(s => {
          const w = Number(String(e.target.value).replace(",", "."));
          if (e.target.value === "" || !Number.isFinite(w)) return;   // not a number: back to what it was
          const clamped = Math.min(Math.max(w, 0), 10);
          if (clamped === 1) delete s.weights[String(l.id)]; else s.weights[String(l.id)] = clamped;
        }) }),
    ))),
    heading("Default muhenkan layout (無変換)"),
    select(api.catalog.layouts.filter(l => !l.henkan).map(l => [l.id, `${l.name}  (${l.id})`]), s.default_muhenkan,
      v => change(s => { s.default_muhenkan = Number(v); })),
    heading("Caps Lock"),
    select(CAPS_LOCK, s.caps_lock, v => change(s => { s.caps_lock = v; }), { "aria-label": "What Caps Lock does" }),
    select(S.CAPS_RULE, s.caps_rule, v => change(s => { s.caps_rule = v; }), { "aria-label": "What Caps Lock types in muhenkan" }),
    hint("Under Windows' Japanese keyboard layout the Caps Lock key alone is 英数 (muhenkan), and ⇧ + it is Caps Lock. Switch mode turns it into a mode key on any layout."),
  ];
}

function profilesPage(s) {
  const on = s.use_profiles;
  const out = [
    heading("Henkan profiles"),
    check("変換, カタカナ and かな select their own henkan candidates", on, v => change(s => { s.use_profiles = v; })),
  ];
  for (const [name] of S.PROFILES) {
    const linkSelect = select([["", "Its own candidates"], ...S.PROFILES.filter(p => p[0] !== name).map(([o]) => [o, `Same as ${WIN_KEY[o]}`])],
      S.link(s, name) ?? "", v => change(s => {
        const link = v || null;
        const p = s.profiles[name] ?? { link: null, enabled: [] };
        if (link == null && S.link(s, name) != null && S.own(s, name).length === 0 && name !== S.MAC_KANA) {
          p.enabled = [...s.enabled];   // a new own list starts as かな's
        }
        p.link = link;
        s.profiles[name] = p;
        ui.editing = name;
      }), { disabled: !on, "aria-label": `Candidates of the ${WIN_KEY[name]} profile` });
    out.push(row(`${WIN_KEY[name]}  (profile-${name})`, linkSelect));
    out.push(h("div", { class: "row indent muted" }, KEY_NOTE[name]));
    if (name !== S.MAC_KANA) {
      out.push(h("div", { class: "row indent" }, "かな in this profile",
        select(S.KANA_IN_PROFILE, s.kana_in_profile[name] ?? "next-or-mac-kana",
          v => change(s => { s.kana_in_profile[name] = v; }), { disabled: !on })));
    }
  }
  out.push(hint("On Windows the profile keys are fixed: 変換 brings up the henkan profile, カタカナ/ひらがな the katakana profile, and an Apple keyboard's かな the mac-kana profile. In henkan, the key of the profile in use moves through the menu (⇧: back). The candidates of mac-kana are the Mac's “かな” list: the profile every other one links to by default."));
  const cycling = Object.keys(S.PROFILE_KEY).some(n => n !== S.MAC_KANA && s.kana_in_profile[n] === "cycle");
  const at = Math.max(0, ORDERS.findIndex(o => o.join() === s.profile_order.join()));
  out.push(row("Cycle order", select(ORDERS.map((o, i) => [i, o.map(n => WIN_KEY[n]).join(" → ")]), at,
    v => change(s => { s.profile_order = ORDERS[Number(v)]; }), { disabled: !on || !cycling })));
  return out;
}

function menuPage(s) {
  return [
    heading("Emoji skin tones (Symbols, 150)"),
    h("input", { type: "text", value: s.skin_tones.join(", "), style: "width:min(420px,100%)", "aria-label": "Emoji skin tone order",
      onchange: e => change(s => {
        const seen = new Set();
        s.skin_tones = e.target.value.toLowerCase().split(/[,\s]+/).filter(t => S.ALL_TONES.includes(t) && !seen.has(t) && seen.add(t));
      }) }),
    hint("The order emoji with skin tones are offered in: none, light, medium-light, medium, medium-dark, dark. Leave one out to hide it."),
    heading("Negative menu entries (⇧Tab from the top)"),
    check("Script forms below the top entry: −1 hiragana, −2 katakana (off: katakana is a ranked entry)", s.script_forms,
      v => change(s => { s.script_forms = v; })),
    select([[-1, "The lowest enabled method that offers them"], ...api.catalog.layouts.filter(l => l.henkan).map(l => [l.id, `${l.name}  (${l.id})`])],
      s.negative_method ?? -1, v => change(s => { s.negative_method = Number(v) < 0 ? null : Number(v); }), { disabled: !s.script_forms }),
    hint("Japanese (Kana) and Japanese (Romaji) offer them: −1 hiragana, −2 katakana."),
    heading("Korean (bada roman, 15)"),
    Object.assign(check("Accept Japanese romaji readings as shortcuts when it saves keystrokes", s.ja_shortcuts,
      v => change(s => { s.ja_shortcuts = v; })), { title: "i.e. mei as 명, or jou as 정" }),
  ];
}

function fKeysPage(s) {
  const { own, keys } = S.fKeys(s, ui.fEditing);
  const isKana = ui.fEditing === S.MAC_KANA;
  const editable = isKana || own;
  const set = f => change(s => {
    const target = isKana ? { intercept: s.intercept_f, keys: s.fkeys } : s.profiles[ui.fEditing]?.f_keys;
    if (!target) return;
    f(target);
    if (isKana) { s.intercept_f = target.intercept; s.fkeys = target.keys; }
  });
  const actions = [["swallow", "Swallowed (does nothing)"], ["pass", "Passed through to the app"],
    ...api.catalog.actions.map(a => [`${a.method}.${a.action}`, `${layoutName(a.method)}: ${a.label}`])];
  return [
    heading("F6–F10"),
    row("F keys of", select(profileOptions(), ui.fEditing, v => { ui.fEditing = v; draw(); }, { disabled: !s.use_profiles }),
      isKana ? null : select([["0", `Same as ${WIN_KEY[S.MAC_KANA]}'s F keys`], ["1", "Its own F keys"]], own ? "1" : "0", v => change(s => {
        const p = s.profiles[ui.fEditing] ?? { link: S.MAC_KANA, enabled: [] };
        // Its own: starts as a copy of かな's. Same as かな's: its own are dropped.
        if (v === "1") p.f_keys = { intercept: s.intercept_f, keys: { ...s.fkeys } }; else delete p.f_keys;
        s.profiles[ui.fEditing] = p;
      }))),
    hint("Each henkan profile can bind F6–F10 its own way, or turn them off, while it's in use. Profiles need Henkan profiles on (Profiles tab)."),
    check("Intercept F6–F10 (off: every F key goes to the app)", keys.intercept, v => set(f => { f.intercept = v; }), !editable),
    ...[6, 7, 8, 9, 10].map(n => row(h("span", { style: "width:2.5em" }, `F${n}`),
      select(actions, keys.keys[`F${n}`] ?? "swallow", v => set(f => {
        f.keys = { ...f.keys };
        if (v === "swallow") delete f.keys[`F${n}`]; else f.keys[`F${n}`] = v;
      }), { disabled: !editable || !keys.intercept, "aria-label": `Action of F${n}` }))),
    h("div", { class: "row" }, h("button", { disabled: !editable, onclick: () => set(f => {
      f.intercept = true;
      f.keys = { F6: "20.hiragana", F7: "20.katakana", F8: "20.halfwidth-katakana", F9: "20.fullwidth-latin", F10: "20.halfwidth-latin" };
    }) }, "Japanese IME preset (F6–F10, Japanese Kana)")),
    hint("⇧, Alt and Ctrl with an F key: up to the bound method (bada defines none, so they're swallowed)."),
  ];
}

function download(name, text) {
  const a = h("a", { href: URL.createObjectURL(new Blob([text], { type: "application/json" })), download: name });
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}
function pickFile(onText) {
  const input = h("input", { type: "file", accept: ".json,application/json", onchange: async () => {
    const file = input.files[0];
    if (file) onText(await file.text());
  } });
  input.click();
}

async function saveDictionary(text) {
  const r = await call("PUT", "/api/dictionary", text);
  api.dictStatus = r.status;
  if (r.ok) api.dictionary = text;
  draw();
}

function dictionaryPage() {
  const text = h("textarea", { class: "code", value: api.dictionary, placeholder: '{"format": "bada-dictionary/1", …}', "aria-label": "Your dictionary (JSON)" });
  const status = api.dictStatus;
  return [
    heading("Your dictionary"),
    h("div", { class: status.startsWith("Loaded") ? "ok" : status.startsWith("No") ? "muted" : "err" }, status),
    hint(`Positive and negative words, as JSON (docs/dictionary-contract.md), kept in ${api.dictionaryPath}. Saved only if it loads; apps pick it up at their next focus change.`),
    text,
    row(h("button", { class: "primary", onclick: () => saveDictionary(text.value.trim()) }, "Save"),
      h("button", { onclick: () => pickFile(t => { text.value = t.trim(); }) }, "Open File…"),
      h("button", { onclick: () => saveDictionary("") }, "Remove"),
      h("button", { onclick: () => call("POST", "/api/open-dictionary-folder") }, "Open Dictionary Folder"),
      link("https://github.com/team-mirae/pado-ime/blob/main/docs/dictionary-contract.md", "The contract")),
  ];
}

function backupPage() {
  return [
    heading("Settings file"),
    row(h("button", { onclick: () => download("bada-settings.json", S.exported(api.settings)) }, "Export Settings…"),
      h("button", { onclick: () => pickFile(t => {
        try {
          const s = S.imported(t);
          ui.editing = S.MAC_KANA;
          ui.note = "Imported.";
          change(x => Object.assign(x, s));
        } catch (e) {
          ui.note = `Couldn't import the settings: ${e.message}`;
          draw();
        }
      }) }, "Import Settings…"),
      h("button", { onclick: () => { ui.note = "Back to the defaults."; change(x => Object.assign(x, S.defaults(), { caps_lock: "respect" })); } }, "Reset to Defaults")),
    ui.note ? hint(ui.note) : null,
    hint("Everything but the Windows page, as JSON (bada-settings/1): the same file bada ime exports and imports on the Mac and at bada.sjun.me. Importing replaces it all. (Recorded profile keys in a Mac file are kept, but Windows uses its own fixed keys.)"),
  ];
}

function windowsPage() {
  const w = api.windows;
  return [
    heading("Keys pado passes on"),
    select([["jis", "Typed as on a JIS keyboard (bada's legends, whatever Windows' layout is)"], ["windows", "Typed by Windows' keyboard layout"]],
      w.pass_on, v => changeWindows(w => { w.pass_on = v; })),
    hint("A letter or symbol key the engine doesn't take (in muhenkan, and keys henkan passes on)."),
    heading("An Apple keyboard's かな key"),
    select([["mac-kana", "Henkan, mac-kana profile"], ["henkan", "Henkan, henkan profile (as 変換)"], ["katakana", "Henkan, katakana profile"], ["none", "Passed to the app"]],
      w.kana_key, v => changeWindows(w => { w.kana_key = v; })),
    heading("Other programs"),
    check("Show the state to other programs (AutoHotkey and the like; never in password fields)", w.publish, v => changeWindows(w => { w.publish = v; })),
    check("Diagnostic log (%ProgramData%\\bada-windows\\pado.log: activations, focus and errors, never what you type)", w.log, v => changeWindows(w => { w.log = v; })),
    hint("Turning pado off in one program, and switching mode or profile by hand: the indicator's menu (right-click the 파 / A in the taskbar)."),
  ];
}

function aboutPage() {
  return [
    h("div", { class: "about" },
      h("img", { src: "icon.svg", alt: "", width: 128, height: 128 }),
      h("h2", {}, "pado ime"),
      h("p", {}, "파도 입력기 · パドIME"),
      h("p", {}, `Settings ${api.version || "?"} · bada engine ${api.engineVersion || "?"}`),
      h("p", { class: "muted" }, "bada ime on Windows: a fork of team-mirae/bada-ime, hosted by bada-windows.")),
    row(link("https://github.com/team-mirae/pado-ime", "GitHub"), link("https://github.com/team-mirae/bada-ime", "bada ime")),
    hint("MIT © Mirae Research. bada-windows: CC0. Data: see the licenses in the install folder."),
  ];
}

// ── Start ────────────────────────────────────────────────────────────────────────────────────────

addEventListener("pagehide", () => navigator.sendBeacon(base + "/api/bye", ""));
setInterval(() => fetch(base + "/api/ping", { method: "POST" }).catch(() => status("Closed. Open Settings again from pado's menu.", true)), 5000);
load().then(draw, () => {});
