// bada-windows' own identifiers, the same for every IME built on it. Each IME's CLSID and language
// profile are its manifest's ("windows": clsid, profile): permanent, in users' registries and
// language lists. kLangBarItemInputMode and kPropInputScope are Windows' own.
#pragma once

#include <guiddef.h>

namespace bw {

// Display attribute of unconfirmed text (underline). {A63A6C6C-856C-4459-A236-1757A9069EC2}
inline constexpr GUID kDisplayAttributeInput = {
    0xa63a6c6c, 0x856c, 0x4459, {0xa2, 0x36, 0x17, 0x57, 0xa9, 0x06, 0x9e, 0xc2}};

// Display attribute of a menu entry other than entry 0 shown in place (thick underline).
// {FACA02D7-F5C1-4F7F-97F5-15BE938F4105}
inline constexpr GUID kDisplayAttributeConverted = {
    0xfaca02d7, 0xf5c1, 0x4f7f, {0x97, 0xf5, 0x15, 0xbe, 0x93, 0x8f, 0x41, 0x05}};

// The input indicator's mode button (Windows 8+), Windows' value (Microsoft's SampleIME).
// {2C77A81E-41CC-4178-A3A7-5F8A987568E6}
inline constexpr GUID kLangBarItemInputMode = {
    0x2c77a81e, 0x41cc, 0x4178, {0xa3, 0xa7, 0x5f, 0x8a, 0x98, 0x75, 0x68, 0xe6}};

// The text property holding a field's input scopes (InputScope.h's GUID_PROP_INPUTSCOPE).
// {1713DD5A-68E7-4A5B-9AF6-592A595C778D}
inline constexpr GUID kPropInputScope = {
    0x1713dd5a, 0x68e7, 0x4a5b, {0x9a, 0xf6, 0x59, 0x2a, 0x59, 0x5c, 0x77, 0x8d}};

}  // namespace bw
