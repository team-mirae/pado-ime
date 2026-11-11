// Adapted from team-mirae/jis-moonshot windows/src/display_attribute.*.
// Display attributes: how apps underline the unconfirmed text.
#pragma once

#include <msctf.h>

namespace bw {

IEnumTfDisplayAttributeInfo* NewDisplayAttributeEnum();
ITfDisplayAttributeInfo* NewDisplayAttributeInfo(REFGUID guid);  // nullptr if not ours

}  // namespace bw
