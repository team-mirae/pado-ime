// The settings dialog: mac/Sources/SettingsWindow.swift's pages, the same settings and the same
// rules, plus a Keyboard page for this page's own options. Changes apply at once.

import * as S from "./settings.js";

const $ = id => document.getElementById(id);
const PAGES = ["Layouts", "Profiles", "Menu", "F Keys", "Dictionary", "Backup", "Keyboard", "About"];
/** Every order of the three profiles, for the cycle order. */
const ORDERS = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]].map(o => o.map(i => S.PROFILES[i][0]));

const ui = { tab: "Layouts", editing: S.MAC_KANA, fEditing: S.MAC_KANA, recording: null, note: "" };
let api;

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

/** Changes the settings (a copy, `f` edits it) and redraws the page. */
function change(f) {
  const s = structuredClone(api.settings);
  f(s);
  api.settings = s;
  draw();
}

const layoutName = id => api.catalog.layouts.find(l => l.id === id)?.name ?? `Layout ${id}`;
const weightText = w => String(w ?? 1);

export function openSettings(a) {
  api = a;
  const dialog = $("settings");
  draw();
  dialog.showModal();
  dialog.addEventListener("close", stopRecording, { once: true });
}

function draw() {
  const s = api.settings;
  if (!s.use_profiles) { ui.editing = S.MAC_KANA; ui.fEditing = S.MAC_KANA; }
  $("tabs").replaceChildren(...PAGES.map(p => h("button", {
    role: "tab", "aria-selected": String(p === ui.tab), onclick: () => { ui.tab = p; ui.note = ""; draw(); },
  }, p)));
  const page = { Layouts: layoutsPage, Profiles: profilesPage, Menu: menuPage, "F Keys": fKeysPage,
    Dictionary: dictionaryPage, Backup: backupPage, Keyboard: keyboardPage, About: aboutPage }[ui.tab](s);
  $("pages").replaceChildren(h("div", { class: "page", role: "tabpanel" }, ...page));
}

const profileOptions = () => S.PROFILES.map(([name, key]) => [name, `${key}  (profile-${name})`]);

function layoutsPage(s) {
  const linked = S.link(s, ui.editing) != null;
  const list = S.own(s, ui.editing);
  return [
    heading("Henkan candidates"),
    hint("Every enabled layout reads the keys at once; the readings are ranked as you type."),
    row("Candidates of", select(profileOptions(), ui.editing, v => { ui.editing = v; draw(); }, { disabled: !s.use_profiles })),
    linked ? hint(`This profile uses ${S.PROFILE_KEY[S.link(s, ui.editing)]}'s candidates (Profiles tab).`) : null,
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
    heading("Default muhenkan layout (英数)"),
    select(api.catalog.layouts.filter(l => !l.henkan).map(l => [l.id, `${l.name}  (${l.id})`]), s.default_muhenkan,
      v => change(s => { s.default_muhenkan = Number(v); })),
    heading("Caps Lock"),
    select(S.CAPS_LOCK, s.caps_lock, v => change(s => { s.caps_lock = v; }), { "aria-label": "What Caps Lock does" }),
    select(S.CAPS_RULE, s.caps_rule, v => change(s => { s.caps_rule = v; }), { "aria-label": "What Caps Lock types in muhenkan" }),
    hint("On the Mac this needs Allow 英数 and かな Everywhere (Accessibility) and Input Monitoring. Here, Caps Lock is the page's own: its light is the key below."),
  ];
}

function profilesPage(s) {
  const on = s.use_profiles;
  const out = [
    heading("Henkan profiles"),
    check("変換 and カタカナ select their own henkan candidates", on, v => change(s => { s.use_profiles = v; })),
  ];
  for (const [name, key] of S.PROFILES) {
    const linkSelect = select([["", "Its own candidates"], ...S.PROFILES.filter(p => p[0] !== name).map(([o, k]) => [o, `Same as ${k}`])],
      S.link(s, name) ?? "", v => change(s => {
        const link = v || null;
        const p = s.profiles[name] ?? { link: null, enabled: [] };
        if (link == null && S.link(s, name) != null && S.own(s, name).length === 0 && name !== S.MAC_KANA) {
          p.enabled = [...s.enabled];   // a new own list starts as かな's
        }
        p.link = link;
        s.profiles[name] = p;
        ui.editing = name;
      }), { disabled: !on, "aria-label": `Candidates of the ${key} profile` });
    const parts = [`${key}  (profile-${name})`, linkSelect];
    if (name !== S.MAC_KANA) {
      parts.push(h("span", { class: "muted" }, S.keyTitle(s.profile_keys[name])),
        h("button", { disabled: !on, onclick: () => recordKey(name) }, ui.recording === name ? "Press a Key (esc: Cancel)" : "Record Key…"),
        h("button", { disabled: !on || s.profile_keys[name] == null || ui.recording != null,
          onclick: () => change(s => { delete s.profile_keys[name]; }) }, "Clear"));
    }
    out.push(row(...parts));
    if (name !== S.MAC_KANA) {
      out.push(h("div", { class: "row indent" }, "かな in this profile",
        select(S.KANA_IN_PROFILE, s.kana_in_profile[name] ?? "next-or-mac-kana",
          v => change(s => { s.kana_in_profile[name] = v; }), { disabled: !on })));
    }
  }
  if (ui.note) out.push(hint(ui.note));
  out.push(hint("A PC keyboard's 変換 and カタカナ: record whatever key your keyboard sends for them. In a browser 変換 is usually “Convert” and カタカナ “KanaMode”; the Mac app stores the same key codes, so a settings file works in both."));
  const cycling = Object.keys(S.PROFILE_KEY).some(n => n !== S.MAC_KANA && s.kana_in_profile[n] === "cycle");
  const at = Math.max(0, ORDERS.findIndex(o => o.join() === s.profile_order.join()));
  out.push(row("Cycle order", select(ORDERS.map((o, i) => [i, o.map(n => S.PROFILE_KEY[n]).join(" → ")]), at,
    v => change(s => { s.profile_order = ORDERS[Number(v)]; }), { disabled: !on || !cycling })));
  return out;
}

// Records the next key for a profile. Esc cancels; かな and 英数 keep their jobs and are refused.
function recordKey(name) {
  if (ui.recording === name) return stopRecording();
  stopRecording();
  ui.recording = name;
  ui.note = "";
  recordKey.listener = e => {
    if (["ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight", "CapsLock"].includes(e.code)) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.code === "Lang1" || e.code === "Lang2") { ui.note = "かな and 英数 keep their own jobs: press another key."; draw(); return; }
    const name = ui.recording;
    stopRecording();
    if (e.code !== "Escape") {
      const mac = S.macCode(e.code);
      if (mac == null) ui.note = `“${e.code}” can't be recorded: it has no key code the Mac app knows.`;
      else change(s => { s.profile_keys[name] = mac; });
    }
    draw();
  };
  addEventListener("keydown", recordKey.listener, true);
  draw();
}

function stopRecording() {
  if (recordKey.listener) removeEventListener("keydown", recordKey.listener, true);
  recordKey.listener = null;
  ui.recording = null;
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
      isKana ? null : select([["0", "Same as かな's F keys"], ["1", "Its own F keys"]], own ? "1" : "0", v => change(s => {
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
    hint("⇧, ⌥ and ⌃ with an F key: up to the bound method (bada defines none, so they're swallowed). On a Mac keyboard, F keys may need fn."),
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

function dictionaryPage(s) {
  const text = h("textarea", { class: "code", value: api.dictionary, placeholder: '{"format": "bada-dictionary", "version": 1, "positive": [{"word": "…"}], "negative": []}', "aria-label": "Your dictionary (JSON)" });
  const status = api.dictStatus;
  return [
    heading("Your dictionary"),
    h("div", { class: status.startsWith("Loaded") ? "ok" : status.startsWith("No") ? "muted" : "err" }, status),
    hint("Positive and negative words, as JSON: docs/dictionary-contract.md. Kept in this browser."),
    text,
    row(h("button", { class: "primary", onclick: () => { api.dictionary = text.value.trim(); draw(); } }, "Load"),
      h("button", { onclick: () => pickFile(t => { api.dictionary = t.trim(); draw(); }) }, "Open File…"),
      h("button", { onclick: () => { api.dictionary = ""; draw(); } }, "Clear"),
      h("a", { href: "https://github.com/team-mirae/bada-ime/blob/main/docs/dictionary-contract.md", target: "_blank", rel: "noopener" }, "The contract")),
  ];
}

function aboutPage() {
  return [
    h("div", { class: "about" },
      h("img", { src: "icon.svg", alt: "", width: 128, height: 128 }),
      h("h2", {}, "bada ime"),
      h("p", {}, "바다 입력기 · バダIME"),
      h("p", {}, `Version ${api.version || "?"}`)),
    row(h("a", { href: "https://github.com/team-mirae/bada-ime", target: "_blank", rel: "noopener" }, "GitHub"),
      h("a", { href: "https://bada.sjun.me" }, "bada.sjun.me")),
    hint("MIT © Mirae Research"),
  ];
}

function backupPage() {
  return [
    heading("Settings file"),
    row(h("button", { onclick: () => download("bada-settings.json", S.exported(api.settings)) }, "Export Settings…"),
      h("button", { onclick: () => pickFile(t => {
        try {
          api.settings = S.imported(t);
          ui.editing = S.MAC_KANA;
          ui.note = "Imported.";
        } catch (e) {
          ui.note = `Couldn't import the settings: ${e.message}`;
        }
        draw();
      }) }, "Import Settings…"),
      h("button", { onclick: () => { api.settings = S.defaults(); ui.note = "Back to the defaults."; draw(); } }, "Reset to Defaults")),
    ui.note ? hint(ui.note) : null,
    hint("Everything but the Keyboard page, as JSON (bada-settings/1): the same file the Mac app exports and imports. Importing replaces it all. Settings are kept in this browser's local storage."),
  ];
}

function keyboardPage() {
  const web = api.web;
  const setWeb = f => { const w = { ...api.web }; f(w); api.web = w; draw(); };
  return [
    heading("Keyboard"),
    row("Shown as", select([["hhkb", "HHKB JIS"], ["apple", "Apple-style JIS (MacBook)"]], web.keyboard, v => setWeb(w => { w.keyboard = v; }))),
    check("JIS legends (small, grey) on the keycaps", web.jis, v => setWeb(w => { w.jis = v; })),
    check("Tap left ⌘ alone for 英数, right ⌘ alone for かな (for keyboards without them)", web.cmdTaps, v => setWeb(w => { w.cmdTaps = v; })),
    row("Appearance", select([["auto", "As the system"], ["light", "Light"], ["dark", "Dark"]], web.theme, v => setWeb(w => { w.theme = v; }))),
    hint("This page's own options: kept in this browser, not in the settings file."),
  ];
}
