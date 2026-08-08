add_library(machina_v8_bridge STATIC
    src/machina_v8_bridge.cpp
)

target_compile_features(machina_v8_bridge PUBLIC cxx_std_20)
target_include_directories(machina_v8_bridge
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/include
)
set_target_properties(machina_v8_bridge PROPERTIES
    POSITION_INDEPENDENT_CODE ON
)

if(MSVC)
    target_compile_options(machina_v8_bridge PRIVATE /W4 /WX)
else()
    target_compile_options(machina_v8_bridge PRIVATE -Wall -Wextra -Wpedantic -Werror)
endif()
