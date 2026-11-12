//! bada-compatible v1.1 `P_state` (§12): `staged` is what a flush would commit, in every layout and
//! state, and asking changes nothing.

use std::ffi::{CStr, CString};
use std::sync::Arc;

use bada::engine::Mode;
use bada::ffi::{self, State, STATE_FORMAT};
use bada::layout::CATALOG;
use bada::{Data, Engine, Key, Mods, Settings};

fn data() -> Arc<Data> {
    let dir = env!("CARGO_MANIFEST_DIR");
    let read = |f: &str| std::fs::read_to_string(format!("{dir}/data/{f}")).unwrap();
    let bin = |f: &str| std::fs::read(format!("{dir}/data/{f}")).unwrap();
    Arc::new(
        Data::parse(&read("ko-words.tsv"), &read("en-words.txt"), &read("hanja.tsv"), &read("oecd.tsv"))
            .with_japanese(&read("ja-words.txt"))
            .with_kanji(&bin("kanji.bin"))
            .with_lm(&bin("ko-jamo-4gram.bin"))
            .with_search(&read("symbols.tsv"), &read("emoji.tsv"), &read("unicode-names.tsv")),
    )
}

const ALT: Mods = Mods { shift: false, ctrl: false, alt: true, meta: false };

/// `staged` equals what a flush commits (on a clone), and asking twice gives the same answer.
fn check(e: &Engine, what: &str) {
    let staged = e.staged();
    assert_eq!(e.staged(), staged, "{what}: asking twice");
    assert_eq!(e.clone().flush().commit, staged, "{what}: staged is what flush commits");
}

fn press(e: &mut Engine, key: Key, mods: Mods, what: &str) {
    e.key(key, mods);
    check(e, &format!("{what} after {key:?}{}", if mods.shift { " ⇧" } else if mods.alt { " ⌥" } else { "" }));
}

#[test]
fn staged_is_what_flush_commits_in_every_layout() {
    use Key::*;
    let d = data();
    let sequences: &[&[(Key, Mods)]] = &[
        &[(KeyD, Mods::NONE), (KeyH, Mods::NONE), (KeyG, Mods::NONE), (KeyR, Mods::NONE)],
        // menu moves, down and back up past the top into the negative entries (script forms)
        &[(KeyK, Mods::NONE), (KeyA, Mods::NONE), (Tab, Mods::NONE), (Tab, Mods::NONE), (ArrowUp, Mods::NONE),
          (Tab, Mods::SHIFT), (Tab, Mods::SHIFT), (Tab, Mods::SHIFT), (Tab, Mods::SHIFT)],
        &[(KeyN, Mods::SHIFT), (KeyI, Mods::NONE), (KeyH, Mods::NONE), (KeyO, Mods::NONE), (KeyN, Mods::NONE),
          (Backspace, Mods::NONE), (Space, Mods::NONE), (KeyA, Mods::NONE)],
        &[(Semicolon, Mods::NONE), (Comma, Mods::NONE), (Digit1, Mods::NONE), (Quote, Mods::SHIFT), (Escape, Mods::NONE)],
    ];
    for &(id, name, henkan) in CATALOG {
        for (i, keys) in sequences.iter().enumerate() {
            let mut e = Engine::new(d.clone());
            let mut s = Settings::default();
            s.script_forms = true;
            if henkan {
                s.enabled = vec![id];
            } else {
                s.default_muhenkan = id;
            }
            e.set_settings(s);
            e.set_mode(if henkan { Mode::Henkan } else { Mode::Muhenkan });
            check(&e, &format!("{name} ({id}) idle"));
            for &(k, m) in keys.iter() {
                press(&mut e, k, m, &format!("{name} ({id}) sequence {i}"));
            }
        }
    }
}

#[test]
fn staged_with_variants_and_picks() {
    use Key::*;
    let mut e = Engine::new(data());
    // ⌥ + a letter's variants in muhenkan: the letter, then a chosen variant
    e.set_mode(Mode::Muhenkan);
    press(&mut e, KeyE, ALT, "variants");
    assert_eq!(e.staged(), "e");
    press(&mut e, ArrowRight, Mods::NONE, "variants");
    press(&mut e, ArrowRight, Mods::NONE, "variants");
    assert_ne!(e.staged(), "e");
    e.flush();
    // a hanja pick on staged text, then on the host's selection
    e.set_mode(Mode::Henkan);
    let mut s = Settings::default();
    s.enabled = vec![13];
    s.fkeys.insert("F7".into(), "13.hanja".into());
    e.set_settings(s);
    for k in [KeyV, KeyH, KeyD, KeyD, Digit9] {
        press(&mut e, k, Mods::NONE, "학교");
    }
    let out = e.key(F7, Mods::NONE);
    check(&e, "hanja on staged text");
    // the menu shows 學校, but a flush (focus lost) commits the staged text, not the pick
    assert_eq!((out.preedit.as_str(), e.staged().as_str()), ("學校", "학교"));
    press(&mut e, Tab, Mods::NONE, "hanja on staged text");
    e.flush();
    press(&mut e, F7, Mods::NONE, "hanja, asking for the selection");
    e.host_selection("학");
    check(&e, "hanja on the selection");
    assert_eq!(e.staged(), "", "a pick on the host's selection commits nothing on a flush");
    press(&mut e, Tab, Mods::NONE, "hanja on the selection");
    assert_eq!(e.staged(), "");
}

fn state(h: *mut ffi::Handle) -> State {
    let p = ffi::bada_state(h);
    serde_json::from_str(unsafe { CStr::from_ptr(p) }.to_str().unwrap()).unwrap()
}

fn commit(p: *const std::ffi::c_char) -> String {
    let v: serde_json::Value = serde_json::from_str(unsafe { CStr::from_ptr(p) }.to_str().unwrap()).unwrap();
    v["commit"].as_str().unwrap_or_default().to_string()
}

#[test]
fn bada_state_over_the_abi() {
    let d = Arc::into_raw(data());
    let h = ffi::bada_new(d);
    let fresh = state(h);
    assert_eq!((fresh.format.as_str(), fresh.mode, fresh.staged.as_str()), (STATE_FORMAT, None, ""), "no mode before the first set_mode");
    ffi::bada_set_mode(h, 1);
    let key = |k: &str| {
        let e = CString::new(format!(r#"{{"key":"{k}"}}"#)).unwrap();
        ffi::bada_key(h, e.as_ptr());
    };
    // 학교 on the default layout, bada plain (16): ㅎ ㅏ ㄱ ㄱ ㅛ
    for k in ["KeyH", "KeyF", "KeyK", "KeyK", "KeyX"] {
        key(k);
    }
    let s = state(h);
    assert_eq!(s.mode.as_deref(), Some("henkan"));
    assert_eq!(s.profile.as_deref(), Some("mac-kana"));
    assert_eq!(s.staged, "학교");
    assert_eq!(s.preedit, "학교", "display as the last Output");
    assert_eq!(state(h), s, "asking changes nothing");
    assert_eq!(commit(ffi::bada_flush(h)), s.staged, "staged is what bada_flush commits");
    let after = state(h);
    assert_eq!((after.staged.as_str(), after.preedit.as_str()), ("", ""));
    ffi::bada_set_mode(h, 0);
    assert_eq!(state(h).mode.as_deref(), Some("muhenkan"));
    assert!(ffi::bada_state(std::ptr::null_mut()).is_null());
    ffi::bada_free(h);
    ffi::bada_data_free(d);
}
