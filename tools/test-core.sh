#!/bin/sh
# Builds and runs the core tests on macOS or Linux, against shout from a bada-compatible checkout
# ($BADA_COMPATIBLE, default ../bada-compatible), built here if needed.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
compat=${BADA_COMPATIBLE:-$root/../bada-compatible}
(cd "$compat" && cargo build --release --lib --quiet)
case "$(uname)" in Darwin) lib=libshout.dylib ;; *) lib=libshout.so ;; esac
out=$root/build/host
mkdir -p "$out"
${CXX:-c++} -std=c++20 -O1 -g -Wall -Wextra -Werror -fsanitize=address,undefined \
  -I"$root/src/core" "$root"/src/core/*.cpp "$root/tests/core_tests.cpp" -o "$out/bw_core_tests" -ldl 2>/dev/null ||
${CXX:-c++} -std=c++20 -O1 -g -Wall -Wextra -Werror -fsanitize=address,undefined \
  -I"$root/src/core" "$root"/src/core/*.cpp "$root/tests/core_tests.cpp" -o "$out/bw_core_tests"
"$out/bw_core_tests" "$compat/target/release/$lib"
