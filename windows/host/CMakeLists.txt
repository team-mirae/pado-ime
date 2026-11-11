# bada-windows: the TSF text input processor (bada_windows.dll) that hosts a bada-compatible engine,
# its portable core and tests. tools/build.ps1 builds it on Windows; tools/test-core.sh runs the core
# tests on macOS / Linux. The engine is not linked: the DLL loads it at run time from its manifest.
cmake_minimum_required(VERSION 3.25)
project(bada_windows LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
file(STRINGS "${CMAKE_CURRENT_SOURCE_DIR}/VERSION" BW_VERSION LIMIT_COUNT 1)
string(REPLACE "." "," BW_VERSION_NUMBERS "${BW_VERSION},0")
add_compile_definitions(BW_VERSION="${BW_VERSION}")

if(MSVC)
    # The static CRT: the DLL loads into every app and can't count on a VC++ runtime.
    set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreaded)
    add_compile_options(/W4 /utf-8 /permissive- /EHsc)
    add_compile_definitions(_CRT_SECURE_NO_WARNINGS)
else()
    add_compile_options(-Wall -Wextra)
endif()
if(WIN32)
    add_compile_definitions(UNICODE _UNICODE NOMINMAX WIN32_LEAN_AND_MEAN _WIN32_WINNT=0x0A00)
endif()

add_library(bw_core STATIC
    src/core/engine.cpp src/core/host.cpp src/core/json.cpp src/core/keys.cpp
    src/core/manifest.cpp src/core/output.cpp src/core/state_table.cpp)
target_include_directories(bw_core PUBLIC src/core)
if(NOT WIN32)
    target_link_libraries(bw_core PUBLIC ${CMAKE_DL_LIBS})
endif()

enable_testing()
add_executable(bw_core_tests tests/core_tests.cpp)
target_link_libraries(bw_core_tests PRIVATE bw_core)
set(BW_SHOUT_LIB "" CACHE FILEPATH "shout's library, for the engine tests (tools/build.ps1 builds it)")
add_test(NAME bw_core_tests COMMAND bw_core_tests "${BW_SHOUT_LIB}")

if(WIN32)
    if(MSVC)
        enable_language(RC)
        set(BW_ICON "${CMAKE_CURRENT_SOURCE_DIR}/icons/bada-windows.ico")
        configure_file(src/tip/bada_windows.rc.in "${CMAKE_CURRENT_BINARY_DIR}/bada_windows.rc" @ONLY)
        set(BW_RC "${CMAKE_CURRENT_BINARY_DIR}/bada_windows.rc")
    endif()
    add_library(bada_windows SHARED
        src/tip/api.cpp src/tip/candidate_window.cpp src/tip/display_attribute.cpp src/tip/dllmain.cpp
        src/tip/edit_session.cpp src/tip/lang_bar.cpp src/tip/log.cpp src/tip/product.cpp
        src/tip/registration.cpp src/tip/settings.cpp src/tip/state_shared.cpp src/tip/text_service.cpp
        src/tip/bada_windows.def ${BW_RC})
    target_include_directories(bada_windows PRIVATE src/tip include)
    target_link_libraries(bada_windows PRIVATE bw_core ole32 oleaut32 user32 gdi32 advapi32 shell32 uuid)

    # A window with a classic Edit, a RichEdit through the IMM layer and a RichEdit on TSF, for typing
    # through the installed TIP by hand or by tools/type-test.ps1; and the API reader.
    add_executable(bw_testpad WIN32 tests/testpad.cpp)
    target_include_directories(bw_testpad PRIVATE include)
    target_link_libraries(bw_testpad PRIVATE ole32 user32 shell32 imm32)
    add_executable(bw_unloadtest tests/unload.cpp)
    target_link_libraries(bw_unloadtest PRIVATE ole32 psapi)
    add_executable(bw_state tests/state_reader.cpp)
    target_include_directories(bw_state PRIVATE include)
endif()
