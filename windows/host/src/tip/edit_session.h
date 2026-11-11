// Adapted from team-mirae/jis-moonshot windows/src/edit_session.*.
// TSF edit sessions: the only way a TIP may change the document.
#pragma once

#include <msctf.h>

#include <functional>
#include <string_view>

namespace bw {

// Runs `body` with a read-write edit cookie: synchronously when TSF allows it (always inside key
// handlers), so text lands before the key that caused it is passed on; otherwise as soon as it can.
// `async`, if given, is set when TSF queued the session to run after this returns: `body` must then
// hold references to whatever it uses.
HRESULT RunEditSession(ITfContext* context, TfClientId client_id,
                       std::function<HRESULT(TfEditCookie)> body, bool* async = nullptr);

// Runs `body` with a read-only cookie, synchronously or not at all (`body` may then capture by
// reference). Fails when TSF can't lock the document at once.
HRESULT RunReadSessionNow(ITfContext* context, TfClientId client_id,
                          std::function<HRESULT(TfEditCookie)> body);

// Inserts `text` at the selection (replacing it) and puts the caret after it.
HRESULT InsertAtSelection(TfEditCookie ec, ITfContext* context, std::wstring_view text);

// Puts the caret at the end of `range`.
HRESULT CaretToEnd(TfEditCookie ec, ITfContext* context, ITfRange* range);

// True when the context's selection lies within `range` (both ends inclusive).
bool CaretInside(TfEditCookie ec, ITfContext* context, ITfRange* range);

}  // namespace bw
