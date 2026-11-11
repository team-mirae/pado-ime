//! pado-settings.exe: pado's settings window (docs/windows.md, "Settings").
//!
//! bada's macOS settings window (its pages, its rules, its `bada-settings/1` file) as a local page:
//! this program serves it on 127.0.0.1 and opens it in an Edge app window (else the default
//! browser). The page reads and writes the settings through this program, which keeps them in
//! pado's registry key, `HKCU\Software\bada-windows\pado` (windows/host/docs/spec.md §9):
//!
//! | Value | What |
//! |---|---|
//! | `Settings` | the whole settings, bada's JSON (mac Settings.swift, web/src/settings.js) |
//! | `EngineSettings` | what the engine gets: the same with profiles dropped while they're off |
//! | `CapsLock`, `CapsRule` | the host's Caps Lock (spec §6.7 / host spec §5), from `caps_lock` / `caps_rule` |
//! | `PassOn`, `KanaKey`, `PublishState`, `DiagnosticLog` | Windows-only host settings |
//! | `SettingsVersion` | bumped on every write: text services re-read at their next focus change |
//!
//! The page talks to this program only with the random token in its URL, and only through
//! 127.0.0.1 by number (a DNS-rebound name is refused). The program exits when the window closes
//! (the page says bye, or its heartbeat stops) or after a minute if the page never opens.

#![windows_subsystem = "windows"]

use std::ffi::c_void;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const NAME: &str = "pado"; // ime.json's name: the registry key and the dictionary's folder
const VERSION: &str = env!("CARGO_PKG_VERSION");

const INDEX: &str = include_str!("../page/index.html");
const UI: &str = include_str!("../page/ui.js");
const STYLE: &str = include_str!("../page/style.css");
// bada's settings model, from upstream unchanged: the same normalisation, defaults and file format.
const SETTINGS_JS: &str = include_str!("../../../web/src/settings.js");
const ICON: &str = include_str!("../../../docs/images/icon/pado-icon.svg");

// ── Win32 ────────────────────────────────────────────────────────────────────────────────────────

type Hkey = isize;
const HKEY_CURRENT_USER: Hkey = 0x8000_0001u32 as i32 as isize;
const HKEY_LOCAL_MACHINE: Hkey = 0x8000_0002u32 as i32 as isize;
const RRF_RT_REG_SZ: u32 = 0x2;
const RRF_RT_REG_DWORD: u32 = 0x10;
const REG_SZ: u32 = 1;
const REG_DWORD: u32 = 4;

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegGetValueW(key: Hkey, sub: *const u16, value: *const u16, flags: u32, kind: *mut u32, data: *mut c_void, size: *mut u32) -> i32;
    fn RegSetKeyValueW(key: Hkey, sub: *const u16, value: *const u16, kind: u32, data: *const c_void, size: u32) -> i32;
}
#[link(name = "bcrypt")]
unsafe extern "system" {
    fn BCryptGenRandom(alg: *mut c_void, buffer: *mut u8, size: u32, flags: u32) -> i32;
}
#[link(name = "shell32")]
unsafe extern "system" {
    fn ShellExecuteW(hwnd: *mut c_void, op: *const u16, file: *const u16, params: *const u16, dir: *const u16, show: i32) -> isize;
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn key() -> Vec<u16> {
    wide(&format!("Software\\bada-windows\\{NAME}"))
}

fn read_string(name: &str) -> Option<String> {
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let (k, n) = (key(), wide(name));
        let mut size = 0u32;
        // SAFETY: valid strings; a null buffer asks for the size.
        if unsafe { RegGetValueW(root, k.as_ptr(), n.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), std::ptr::null_mut(), &mut size) } != 0 {
            continue;
        }
        let mut buffer = vec![0u16; size as usize / 2 + 1];
        size = (buffer.len() * 2) as u32;
        // SAFETY: the buffer is `size` bytes.
        if unsafe { RegGetValueW(root, k.as_ptr(), n.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buffer.as_mut_ptr().cast(), &mut size) } == 0 {
            let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
            return Some(String::from_utf16_lossy(&buffer[..end]));
        }
    }
    None
}

fn read_dword(name: &str) -> Option<u32> {
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let (k, n) = (key(), wide(name));
        let (mut value, mut size) = (0u32, 4u32);
        // SAFETY: a 4-byte buffer.
        if unsafe { RegGetValueW(root, k.as_ptr(), n.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), (&mut value as *mut u32).cast(), &mut size) } == 0 {
            return Some(value);
        }
    }
    None
}

fn write_string(name: &str, value: &str) -> bool {
    let (k, n, v) = (key(), wide(name), wide(value));
    // SAFETY: a NUL-terminated UTF-16 string of v.len() units.
    unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr(), REG_SZ, v.as_ptr().cast(), (v.len() * 2) as u32) == 0 }
}

fn write_dword(name: &str, value: u32) -> bool {
    let (k, n) = (key(), wide(name));
    // SAFETY: a 4-byte value.
    unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr(), REG_DWORD, (&value as *const u32).cast(), 4) == 0 }
}

/// A new SettingsVersion (windows/host settings.cpp `Changed`): any value but the current one.
fn bump_version() {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let mut version = (now.as_nanos() as u64 ^ (now.as_nanos() >> 32) as u64) as u32;
    if Some(version) == read_dword("SettingsVersion") {
        version = version.wrapping_add(1);
    }
    write_dword("SettingsVersion", version);
}

fn random_token() -> String {
    let mut bytes = [0u8; 24];
    // SAFETY: BCRYPT_USE_SYSTEM_PREFERRED_RNG (2) with no algorithm handle.
    unsafe { BCryptGenRandom(std::ptr::null_mut(), bytes.as_mut_ptr(), bytes.len() as u32, 2) };
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn shell_open(file: &str, params: Option<&str>) -> bool {
    let (op, f) = (wide("open"), wide(file));
    let p = params.map(wide);
    // SAFETY: valid strings or null.
    let r = unsafe {
        ShellExecuteW(std::ptr::null_mut(), op.as_ptr(), f.as_ptr(), p.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()), std::ptr::null(), 1)
    };
    r > 32
}

// ── The settings ─────────────────────────────────────────────────────────────────────────────────

fn dictionary_path() -> std::path::PathBuf {
    let appdata = std::env::var_os("APPDATA").unwrap_or_default();
    std::path::PathBuf::from(appdata).join("bada-windows").join(NAME).join("dictionary.json")
}

fn dictionary_status(summary: &bada::userdict::Summary, present: bool) -> String {
    if !present {
        "No dictionary.json yet.".into()
    } else if summary.ok {
        format!("Loaded: {} positive, {} negative.", summary.positive, summary.negative)
    } else {
        format!("dictionary.json wasn't loaded: {}", summary.error)
    }
}

fn state() -> Value {
    let raw = read_string("Settings")
        .or_else(|| read_string("EngineSettings"))
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .unwrap_or_else(|| json!({}));
    let layouts: Vec<Value> = bada::layout::CATALOG.iter().map(|(id, name, henkan)| json!({"id": id, "name": name, "henkan": henkan})).collect();
    let actions: Vec<Value> = bada::engine::ACTIONS.iter().map(|(m, a, label)| json!({"method": m, "action": a, "label": label})).collect();
    let path = dictionary_path();
    let dictionary = std::fs::read_to_string(&path).ok();
    let summary = bada::userdict::load(dictionary.as_deref().unwrap_or(""));
    json!({
        "settings": raw,
        "catalog": {"layouts": layouts, "actions": actions},
        "dictionary": dictionary.clone().unwrap_or_default(),
        "dictionaryPath": path.to_string_lossy(),
        "dictStatus": dictionary_status(&summary, dictionary.is_some()),
        "version": VERSION,
        "engineVersion": bada_version(),
        "windows": {
            "caps_lock": read_string("CapsLock").unwrap_or_else(|| "respect".into()),
            "caps_rule": read_string("CapsRule").unwrap_or_else(|| "allcaps".into()),
            "pass_on": read_string("PassOn").unwrap_or_else(|| "jis".into()),
            "kana_key": read_string("KanaKey").unwrap_or_else(|| "mac-kana".into()),
            "publish": read_dword("PublishState").unwrap_or(1) != 0,
            "log": read_dword("DiagnosticLog").unwrap_or(0) != 0,
        },
    })
}

fn bada_version() -> &'static str {
    // engine/Cargo.toml's version, at build time.
    option_env!("BADA_ENGINE_VERSION").unwrap_or("")
}

fn one_of<'a>(v: &'a Value, field: &str, allowed: &[&str]) -> Option<&'a str> {
    v.get(field).and_then(Value::as_str).filter(|s| allowed.contains(s))
}

/// `{"settings": {…}, "engine": {…}}`: the page's settings and what the engine takes of them.
fn put_settings(body: &str) -> Result<Value, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let (Some(settings), Some(engine)) = (v.get("settings").filter(|s| s.is_object()), v.get("engine").filter(|s| s.is_object())) else {
        return Err("settings and engine must be objects".into());
    };
    let mut ok = write_string("Settings", &settings.to_string()) && write_string("EngineSettings", &engine.to_string());
    if let Some(caps) = one_of(settings, "caps_lock", &["toggle", "transient", "respect"]) {
        ok &= write_string("CapsLock", caps);
    }
    if let Some(rule) = one_of(settings, "caps_rule", &["allcaps", "uppersnake"]) {
        ok &= write_string("CapsRule", rule);
    }
    bump_version();
    if ok { Ok(json!({"ok": true})) } else { Err("couldn't write to the registry".into()) }
}

/// `{"pass_on", "kana_key", "publish", "log"}`: the Windows page.
fn put_windows(body: &str) -> Result<Value, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    if let Some(p) = one_of(&v, "pass_on", &["jis", "windows"]) {
        write_string("PassOn", p);
    }
    if let Some(k) = one_of(&v, "kana_key", &["mac-kana", "henkan", "katakana", "none"]) {
        write_string("KanaKey", k);
    }
    if let Some(p) = v.get("publish").and_then(Value::as_bool) {
        write_dword("PublishState", p as u32);
    }
    if let Some(l) = v.get("log").and_then(Value::as_bool) {
        write_dword("DiagnosticLog", l as u32);
    }
    bump_version();
    Ok(json!({"ok": true}))
}

/// The dictionary's text: checked by the engine's own parser, then saved ("" removes the file).
/// A file that doesn't load isn't saved.
fn put_dictionary(body: &str) -> Value {
    let path = dictionary_path();
    let text = body.trim();
    if text.is_empty() {
        let _ = std::fs::remove_file(&path);
        return json!({"ok": true, "status": dictionary_status(&Default::default(), false)});
    }
    let summary = bada::userdict::load(text);
    if summary.ok {
        let saved = path.parent().is_some_and(|d| std::fs::create_dir_all(d).is_ok()) && std::fs::write(&path, text).is_ok();
        if !saved {
            return json!({"ok": false, "status": format!("Couldn't save {}.", path.display())});
        }
    }
    json!({"ok": summary.ok, "status": dictionary_status(&summary, true)})
}

// ── The server ───────────────────────────────────────────────────────────────────────────────────

struct Request {
    method: String,
    path: String,
    host: String,
    body: String,
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let mut data = Vec::new();
    let mut buffer = [0u8; 8192];
    let head_end = loop {
        let n = stream.read(&mut buffer).ok()?;
        if n == 0 {
            return None;
        }
        data.extend_from_slice(&buffer[..n]);
        if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        if data.len() > 64 * 1024 {
            return None;
        }
    };
    let head = String::from_utf8_lossy(&data[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let (method, path) = (first.next()?.to_string(), first.next()?.to_string());
    let mut length = 0usize;
    let mut host = String::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "content-length" => length = v.trim().parse().ok()?,
                "host" => host = v.trim().to_string(),
                _ => {}
            }
        }
    }
    if length > 16 * 1024 * 1024 {
        return None;
    }
    let mut body = data[head_end..].to_vec();
    while body.len() < length {
        let n = stream.read(&mut buffer).ok()?;
        if n == 0 {
            return None;
        }
        body.extend_from_slice(&buffer[..n]);
    }
    body.truncate(length);
    Some(Request { method, path, host, body: String::from_utf8_lossy(&body).into_owned() })
}

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\
         X-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

fn json_reply(stream: &mut TcpStream, result: Result<Value, String>) {
    match result {
        Ok(v) => respond(stream, "200 OK", "application/json", v.to_string().as_bytes()),
        Err(e) => respond(stream, "400 Bad Request", "application/json", json!({"ok": false, "error": e}).to_string().as_bytes()),
    }
}

fn edge() -> Option<std::path::PathBuf> {
    ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"]
        .iter()
        .filter_map(std::env::var_os)
        .map(|d| std::path::PathBuf::from(d).join("Microsoft\\Edge\\Application\\msedge.exe"))
        .find(|p| p.exists())
}

fn open_window(url: &str) {
    if let Some(edge) = edge() {
        // A profile of its own: a window of its own, whatever Edge the user has open.
        let profile = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default())
            .join("bada-windows")
            .join(NAME)
            .join("settings-window");
        let started = std::process::Command::new(edge)
            .arg(format!("--app={url}"))
            .arg(format!("--user-data-dir={}", profile.display()))
            .args(["--no-first-run", "--no-default-browser-check", "--window-size=900,760"])
            .spawn()
            .is_ok();
        if started {
            return;
        }
    }
    shell_open(url, None);
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("no local port");
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
    listener.set_nonblocking(true).ok();
    let token = random_token();
    let base = format!("/{token}");
    let host_ok = format!("127.0.0.1:{port}");
    let url = format!("http://127.0.0.1:{port}{base}/");
    // `--url-file PATH`: write the URL there instead of opening a window (for tests).
    let args: Vec<String> = std::env::args().collect();
    match args.iter().position(|a| a == "--url-file").and_then(|i| args.get(i + 1)) {
        Some(file) => {
            let _ = std::fs::write(file, &url);
        }
        None => open_window(&url),
    }

    let started = Instant::now();
    let mut seen: Option<Instant> = None;
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_nonblocking(false).ok();
                let Some(req) = read_request(&mut stream) else { continue };
                if req.host != host_ok {
                    respond(&mut stream, "403 Forbidden", "text/plain", b"forbidden");
                    continue;
                }
                let Some(path) = req.path.strip_prefix(&base) else {
                    respond(&mut stream, "404 Not Found", "text/plain", b"not found");
                    continue;
                };
                seen = Some(Instant::now());
                match (req.method.as_str(), path) {
                    ("GET", "/") => respond(&mut stream, "200 OK", "text/html; charset=utf-8", INDEX.as_bytes()),
                    ("GET", "/ui.js") => respond(&mut stream, "200 OK", "text/javascript; charset=utf-8", UI.as_bytes()),
                    ("GET", "/settings.js") => respond(&mut stream, "200 OK", "text/javascript; charset=utf-8", SETTINGS_JS.as_bytes()),
                    ("GET", "/style.css") => respond(&mut stream, "200 OK", "text/css; charset=utf-8", STYLE.as_bytes()),
                    ("GET", "/icon.svg") => respond(&mut stream, "200 OK", "image/svg+xml", ICON.as_bytes()),
                    ("GET", "/api/state") => json_reply(&mut stream, Ok(state())),
                    ("POST", "/api/ping") => json_reply(&mut stream, Ok(json!({"ok": true}))),
                    ("PUT", "/api/settings") => json_reply(&mut stream, put_settings(&req.body)),
                    ("PUT", "/api/windows") => json_reply(&mut stream, put_windows(&req.body)),
                    ("PUT", "/api/dictionary") => json_reply(&mut stream, Ok(put_dictionary(&req.body))),
                    ("POST", "/api/open-dictionary-folder") => {
                        let folder = dictionary_path().parent().map(|p| p.to_path_buf()).unwrap_or_default();
                        let _ = std::fs::create_dir_all(&folder);
                        json_reply(&mut stream, Ok(json!({"ok": shell_open(&folder.to_string_lossy(), None)})))
                    }
                    ("POST", "/api/open") => {
                        // Only the project's own pages.
                        let url = req.body.trim();
                        let allowed = url.starts_with("https://github.com/team-mirae/");
                        json_reply(&mut stream, Ok(json!({"ok": allowed && shell_open(url, None)})))
                    }
                    ("POST", "/api/bye") => {
                        json_reply(&mut stream, Ok(json!({"ok": true})));
                        return;
                    }
                    _ => respond(&mut stream, "404 Not Found", "text/plain", b"not found"),
                }
            }
            Err(_) => {
                std::thread::sleep(Duration::from_millis(40));
                // The page pings every 5 s while it's open (a hidden page's timers may slow to once a minute),
                // and says bye when it closes.
                let idle = match seen {
                    Some(t) => t.elapsed() > Duration::from_secs(150),
                    None => started.elapsed() > Duration::from_secs(60),
                };
                if idle {
                    return;
                }
            }
        }
    }
}
