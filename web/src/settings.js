// The user's settings, as the macOS shell has them (mac/Sources/Settings.swift): the same JSON, kept
// in localStorage, so a settings file (bada-settings/1) moves between the Mac app and this page.

export const ALL_TONES = ["none", "light", "medium-light", "medium", "medium-dark", "dark"];
export const MAC_KANA = "mac-kana";
/** The henkan profiles (spec §6.6) and the keys that select them. */
export const PROFILES = [["mac-kana", "かな"], ["henkan", "変換"], ["katakana", "カタカナ"]];
export const PROFILE_KEY = Object.fromEntries(PROFILES);

/** What かな does in henkan mode in another profile than mac-kana (KanaInProfile in Settings.swift). */
export const KANA_IN_PROFILE = [
  ["next-or-mac-kana", "Next entry (⇧: previous) if there's a menu, else back to かな's profile"],
  ["mac-kana", "Back to かな's profile"],
  ["cycle", "Cycle the profiles (⇧: backwards)"],
  ["henkan", "To 変換's profile"],
  ["katakana", "To カタカナ's profile"],
  ["muhenkan", "Leave henkan mode (as 英数)"],
];

/** What Caps Lock does (spec §6.7; CapsLock.Setting in mac/Sources/CapsLock.swift). */
export const CAPS_LOCK = [
  ["toggle", "Switch mode (⇧ ⌃ ⌥ ⌘ + Caps Lock: Caps Lock, in muhenkan)"],
  ["transient", "Caps Lock in muhenkan only, forgotten when you leave it"],
  ["respect", "Caps Lock everywhere, until you log out"],
];
/** What Caps Lock types in muhenkan (spec §6.2; CapsLock.Rule). */
export const CAPS_RULE = [
  ["allcaps", "ALL CAPS: letters in capitals"],
  ["uppersnake", "UPPER_SNAKE: capitals, Space types _ (⌥Space a space)"],
];

const STORE = "bada-settings";
const WEB_STORE = "bada-web";

export function defaults() {
  return {
    enabled: [16], default_muhenkan: 0, use_profiles: false, profiles: {}, weights: {}, profile_keys: {},
    kana_in_profile: {}, profile_order: ["mac-kana", "henkan", "katakana"], skin_tones: [...ALL_TONES],
    script_forms: false, negative_method: null, intercept_f: true, fkeys: {}, ja_shortcuts: true,
    caps_lock: "toggle", caps_rule: "allcaps",
  };
}

const isObj = v => v && typeof v === "object" && !Array.isArray(v);
const ints = v => Array.isArray(v) ? v.filter(Number.isInteger) : null;

/** Any JSON object as settings: fields missing or of the wrong type take their defaults. */
export function normalize(raw) {
  const s = defaults();
  if (!isObj(raw)) return s;
  if (ints(raw.enabled)) s.enabled = ints(raw.enabled);
  if (Number.isInteger(raw.default_muhenkan)) s.default_muhenkan = raw.default_muhenkan;
  if (typeof raw.use_profiles === "boolean") s.use_profiles = raw.use_profiles;
  if (isObj(raw.profiles)) {
    for (const [name, p] of Object.entries(raw.profiles)) {
      if (!isObj(p)) continue;
      const profile = { link: typeof p.link === "string" ? p.link : null, enabled: ints(p.enabled) ?? [] };
      if (isObj(p.f_keys)) profile.f_keys = { intercept: p.f_keys.intercept !== false, keys: strings(p.f_keys.keys) };
      s.profiles[name] = profile;
    }
  }
  if (isObj(raw.weights)) for (const [k, v] of Object.entries(raw.weights)) if (Number.isFinite(v)) s.weights[k] = v;
  if (isObj(raw.profile_keys)) for (const [k, v] of Object.entries(raw.profile_keys)) if (Number.isInteger(v)) s.profile_keys[k] = v;
  if (isObj(raw.kana_in_profile)) s.kana_in_profile = strings(raw.kana_in_profile);
  else if (typeof raw.kana_in_profile === "string") s.kana_in_profile = { henkan: raw.kana_in_profile, katakana: raw.kana_in_profile };
  if (Array.isArray(raw.profile_order)) s.profile_order = raw.profile_order.filter(x => typeof x === "string");
  if (Array.isArray(raw.skin_tones)) s.skin_tones = raw.skin_tones.filter(t => ALL_TONES.includes(t));
  if (typeof raw.script_forms === "boolean") s.script_forms = raw.script_forms;
  if (Number.isInteger(raw.negative_method)) s.negative_method = raw.negative_method;
  if (typeof raw.intercept_f === "boolean") s.intercept_f = raw.intercept_f;
  if (isObj(raw.fkeys)) s.fkeys = strings(raw.fkeys);
  if (typeof raw.ja_shortcuts === "boolean") s.ja_shortcuts = raw.ja_shortcuts;
  if (CAPS_LOCK.some(([v]) => v === raw.caps_lock)) s.caps_lock = raw.caps_lock;
  if (CAPS_RULE.some(([v]) => v === raw.caps_rule)) s.caps_rule = raw.caps_rule;
  return s;
}

function strings(o) {
  const out = {};
  if (isObj(o)) for (const [k, v] of Object.entries(o)) if (typeof v === "string") out[k] = v;
  return out;
}

export function load() {
  try { return normalize(JSON.parse(localStorage.getItem(STORE) || "{}")); } catch { return defaults(); }
}

export function save(settings) {
  try { localStorage.setItem(STORE, JSON.stringify(settings)); } catch {}
}

/** The settings as the engine takes them: with profiles off, every profile is かな's. */
export function engineJSON(s) {
  const copy = structuredClone(s);
  if (!copy.use_profiles) copy.profiles = {};
  for (const p of Object.values(copy.profiles)) if (p.link == null) delete p.link;
  if (copy.negative_method == null) delete copy.negative_method;
  return copy;
}

/** The settings file: `{"format": "bada-settings/1", "settings": {…}}`, keys sorted as the Mac writes it. */
export function exported(s) {
  const sorted = v => Array.isArray(v) ? v.map(sorted) : isObj(v)
    ? Object.fromEntries(Object.keys(v).sort().filter(k => v[k] !== null).map(k => [k, sorted(v[k])])) : v;
  return JSON.stringify({ format: "bada-settings/1", settings: sorted(s) }, null, 2);
}

export function imported(text) {
  const file = JSON.parse(text);
  if (!isObj(file) || file.format !== "bada-settings/1" || !isObj(file.settings)) {
    throw new Error("It isn't a bada ime settings file (format bada-settings/1).");
  }
  return normalize(file.settings);
}

/** This page's own settings: not in the settings file. */
export function loadWeb() {
  const web = { keyboard: "hhkb", jis: true, cmdTaps: true, theme: "auto" };
  try { Object.assign(web, JSON.parse(localStorage.getItem(WEB_STORE) || "{}")); } catch {}
  return web;
}
export function saveWeb(web) {
  try { localStorage.setItem(WEB_STORE, JSON.stringify(web)); } catch {}
}

// ── Profiles (spec §6.6) ────────────────────────────────────────────────────────────────────────

/** The profile `name` links to, or null for its own candidates (SettingsWindow.link). */
export function link(s, name) {
  if (s.profiles[name]) return s.profiles[name].link ?? null;
  return name === MAC_KANA ? null : MAC_KANA;
}

/** A profile's own list (SettingsWindow.own). */
export function own(s, name) {
  return name === MAC_KANA ? s.enabled : s.profiles[name]?.enabled ?? [];
}

/** The candidates of a profile, links followed (engine.rs `candidates`), with profiles off: かな's. */
export function candidates(s, name) {
  const profiles = s.use_profiles ? s.profiles : {};
  const names = PROFILES.map(p => p[0]);
  let at = name;
  for (let i = 0; i < names.length; i++) {
    const p = profiles[at];
    let next;
    if (p && p.link != null) next = p.link;
    else if (p && at !== MAC_KANA) return p.enabled;
    else if (!p && at !== MAC_KANA) next = MAC_KANA;
    else return s.enabled;
    if (!names.includes(next)) break;
    at = next;
  }
  return s.enabled;
}

/** The F keys in effect for `name`: its own, or かな's (SettingsWindow.fKeys). */
export function fKeys(s, name) {
  if (name !== MAC_KANA && s.profiles[name]?.f_keys) return { own: true, keys: s.profiles[name].f_keys };
  return { own: false, keys: { intercept: s.intercept_f, keys: s.fkeys } };
}

/**
 * What a language key does in henkan mode (mac/Sources/ProfileKeys.swift): {step: back},
 * {profile: name} or {muhenkan: true}. `key`: the profile of the key pressed (かな: mac-kana).
 */
export function profileKeyAction(s, key, current, shift, menu) {
  if (!s.use_profiles) return { step: shift };
  const names = PROFILES.map(p => p[0]);
  let target = key;
  if (key === MAC_KANA && current !== MAC_KANA) {
    switch (s.kana_in_profile[current] ?? "next-or-mac-kana") {
      case "mac-kana": target = MAC_KANA; break;
      case "cycle": {
        const order = s.profile_order.filter(n => names.includes(n));
        if (!order.length) { target = MAC_KANA; break; }
        const at = Math.max(0, order.indexOf(current));
        target = order[(at + (shift ? order.length - 1 : 1)) % order.length];
        break;
      }
      case "henkan": target = "henkan"; break;
      case "katakana": target = "katakana"; break;
      case "muhenkan": return { muhenkan: true };
      default:
        if (menu) return { step: shift };
        target = MAC_KANA;
    }
  }
  return target === current ? { step: shift } : { profile: target };
}

// ── Recorded profile keys ───────────────────────────────────────────────────────────────────────
// The Mac stores a profile key as its macOS key code, or 0x70000 | HID usage for the PC keys macOS
// doesn't pass on (mac/Sources/HIDKeys.swift). Browsers name both by KeyboardEvent.code.

const HID = 0x70000;
export const MAC_CODES = {
  KeyA: 0, KeyS: 1, KeyD: 2, KeyF: 3, KeyH: 4, KeyG: 5, KeyZ: 6, KeyX: 7, KeyC: 8, KeyV: 9, KeyB: 11, KeyQ: 12,
  KeyW: 13, KeyE: 14, KeyR: 15, KeyY: 16, KeyT: 17, Digit1: 18, Digit2: 19, Digit3: 20, Digit4: 21, Digit6: 22,
  Digit5: 23, Equal: 24, Digit9: 25, Digit7: 26, Minus: 27, Digit8: 28, Digit0: 29, BracketRight: 30, KeyO: 31,
  KeyU: 32, BracketLeft: 33, KeyI: 34, KeyP: 35, Enter: 36, KeyL: 37, KeyJ: 38, Quote: 39, KeyK: 40,
  Semicolon: 41, Backslash: 42, Comma: 43, Slash: 44, KeyN: 45, KeyM: 46, Period: 47, Tab: 48, Space: 49,
  Backquote: 50, Backspace: 51, Escape: 53, NumpadEnter: 76, F17: 64, F18: 79, F19: 80, F20: 90,
  IntlYen: 93, IntlRo: 94, NumpadComma: 95, F5: 96, F6: 97, F7: 98, F3: 99, F8: 100, F9: 101, Lang2: 102,
  F11: 103, Lang1: 104, F13: 105, F16: 106, F14: 107, F10: 109, F12: 111, F15: 113, Help: 114, Home: 115,
  PageUp: 116, Delete: 117, F4: 118, End: 119, F2: 120, PageDown: 121, F1: 122, ArrowLeft: 123,
  ArrowRight: 124, ArrowDown: 125, ArrowUp: 126,
  KanaMode: HID | 0x88, Convert: HID | 0x8a, NonConvert: HID | 0x8b, Lang3: HID | 0x92, Lang4: HID | 0x93, Lang5: HID | 0x94,
};
const CODE_OF = Object.fromEntries(Object.entries(MAC_CODES).map(([c, n]) => [n, c]));
const HID_NAMES = {
  0x87: "ろ (HID 0x87)", 0x88: "カタカナ/ひらがな (HID 0x88)", 0x89: "¥ (HID 0x89)", 0x8a: "変換 (HID 0x8A)",
  0x8b: "無変換 (HID 0x8B)", 0x92: "LANG3 (HID 0x92)", 0x93: "LANG4 (HID 0x93)", 0x94: "LANG5 (HID 0x94)",
};

export const macCode = code => MAC_CODES[code];
export const codeOf = mac => CODE_OF[mac];

export function keyTitle(mac) {
  if (mac == null) return "Key: none";
  if (mac >= HID) return "Key: " + (HID_NAMES[mac - HID] ?? `HID 0x${(mac - HID).toString(16).toUpperCase().padStart(2, "0")}`);
  return "Key: " + (CODE_OF[mac] ?? `code ${mac}`);
}
