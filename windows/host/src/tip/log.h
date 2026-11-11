// The diagnostic log, off by default (DiagnosticLog = 1, docs/settings.md). It records activations,
// focus and errors, never typed text: %ProgramData%\bada-windows\<ime>.log.
#pragma once

namespace bw::log {

void Write(const wchar_t* format, ...);

}  // namespace bw::log
