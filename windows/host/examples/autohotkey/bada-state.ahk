#Requires AutoHotkey v2.0
; Reads a bada-windows IME's state (docs/api.md) and shows it in a tooltip whenever the mode, the
; profile, secure input or composing changes. Ctrl+Alt+S shows what's staged in the focused field.
; CC0. Works from 64- and 32-bit AutoHotkey (the registry gives each its own DLL).

CLSID := "{9C0F2BF6-0B78-447E-A99D-2FBAE9E8995E}"   ; the IME's windows.clsid (this one: shout)

try Dll := RegRead("HKLM\SOFTWARE\Classes\CLSID\" CLSID "\InprocServer32")
catch {
    MsgBox "The IME isn't installed (" CLSID ")."
    ExitApp
}

; The mode in the focused window: 1 henkan, 0 muhenkan, -1 not active, -2 not run yet.
BadaMode(hwnd := 0) => DllCall(Dll "\BadaWindowsQueryWindow", "ptr", hwnd, "int")

; Calls GetState / GetStaged with a big enough buffer. A password field throws: what's typed there is
; never given out (BADA_WINDOWS_E_SECURE, -3).
BadaText(function, hwnd := 0) {
    loop 3 {
        n := DllCall(Dll "\" function, "ptr", hwnd, "ptr", 0, "int", 0, "int")
        if (n = -3)
            throw OSError(5, -1, "bada-windows: secure input (a password field)")  ; ERROR_ACCESS_DENIED
        if (n < 0)
            return ""                        ; not active (-1, -2), not reported (-5), busy (-6)
        buf := Buffer((n + 1) * 2)
        got := DllCall(Dll "\" function, "ptr", hwnd, "ptr", buf, "int", n + 1, "int")
        if (got = -3)
            throw OSError(5, -1, "bada-windows: secure input (a password field)")
        if (got >= 0)
            return StrGet(buf, got, "UTF-16")
        ; -4: it grew between the two calls; ask again
    }
    return ""
}
BadaState(hwnd := 0) => BadaText("BadaWindowsGetState", hwnd)   ; JSON
BadaStaged(hwnd := 0) => BadaText("BadaWindowsGetStaged", hwnd)

; Pushed by the IME: wParam the thread, lParam mode (bits 0-7) and flags (bits 8-15: 1 secure,
; 2 off in the program, 4 composing, 8 staged text not reported).
OnBadaState(wParam, lParam, msg, hwnd) {
    mode := lParam & 0xFF, flags := (lParam >> 8) & 0xFF
    text := (mode = 1 ? "henkan" : "muhenkan") . (flags & 1 ? " · secure input" : "") . (flags & 4 ? " · composing" : "")
    ToolTip text
    SetTimer () => ToolTip(), -1200
}
OnMessage(DllCall(Dll "\BadaWindowsStateMessage", "uint"), OnBadaState)
Persistent

^!s:: {
    try MsgBox "Mode " BadaMode() "`nStaged: [" BadaStaged() "]`n`n" BadaState()
    catch OSError as e
        MsgBox e.Message
}
