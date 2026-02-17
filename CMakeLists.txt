cmake_minimum_required(VERSION 3.15)
project(OpenGGS LANGUAGES CXX)

# Optionally set C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${OpenGGS_SOURCE_DIR}/cmake")

if (WIN32)
    set(Rust_CARGO_TARGET "x86_64-pc-windows-gnu" CACHE STRING "Rust target triple" FORCE)
    set(Rust_CARGO_TARGET_CACHED "x86_64-pc-windows-gnu" CACHE STRING "Rust target triple cached" FORCE)
endif()

# Define a CMake option that defaults to OFF
option(ENABLE_CLANG_TIDY "Enable clang-tidy checks" OFF)

include(FetchContent)
set(FETCHCONTENT_QUIET OFF)
set(FETCHCONTENT_UPDATES_DISCONNECTED OFF)

#------------------------------------------------------------------------------
# SDL2
#------------------------------------------------------------------------------
FetchContent_Declare(
  SDL2
  GIT_REPOSITORY https://github.com/libsdl-org/SDL.git
  GIT_TAG release-2.32.2
)
FetchContent_MakeAvailable(SDL2)

# Disable building SDL2 tests, if desired
set(SDL_TEST OFF CACHE BOOL "Do not build SDL2_test" FORCE)

#------------------------------------------------------------------------------
# SDL2_image
#------------------------------------------------------------------------------
FetchContent_Declare(
  SDL2_image
  GIT_REPOSITORY https://github.com/libsdl-org/SDL_image.git
  GIT_TAG release-2.8.8
)

# Optionally disable tests or set build options
set(SUPPORT_AVIF OFF CACHE BOOL "Disable AVIF support" FORCE)
set(SDL2IMAGE_AVIF OFF CACHE BOOL "Disable TIFF support in SDL2_image" FORCE)
set(SUPPORT_WEBP OFF CACHE BOOL "Disable WEBP support" FORCE)
set(SUPPORT_TIFF OFF CACHE BOOL "Disable TIFF support in SDL2_image" FORCE)
set(SDL2IMAGE_TIF OFF CACHE BOOL "Disable TIFF support in SDL2_image" FORCE)
set(IMAGE_TEST OFF CACHE BOOL "Disable building SDL2_image test" FORCE)

FetchContent_MakeAvailable(SDL2_image)

#------------------------------------------------------------------------------
# SDL2_mixer
#------------------------------------------------------------------------------
FetchContent_Declare(
  SDL2_mixer
  GIT_REPOSITORY https://github.com/libsdl-org/SDL_mixer.git
  GIT_TAG release-2.8.1
)

set(MUSIC_OPUS OFF CACHE BOOL "Disable Opus in SDL2_mixer" FORCE)
set(MUSIC_MP3_SMPEG OFF CACHE BOOL "Disable SMPEG-based MP3" FORCE)
set(MUSIC_MPG123 OFF CACHE BOOL "Disable mpg123-based MP3" FORCE)
set(MIXER_TEST OFF CACHE BOOL "Disable building SDL2_mixer test" FORCE)
set(SDL2MIXER_OPUS OFF CACHE BOOL "Disable building SDL2MIXER_OPUS" FORCE)
set(SDL2MIXER_MOD OFF CACHE BOOL "Disable building SDL2MIXER_MOD" FORCE)
set(SDL2MIXER_MOD_XMP OFF CACHE BOOL "Disable building SDL2MIXER_MOD_XMP" FORCE)
set(MIXER_TEST OFF CACHE BOOL "Disable building SDL2_mixer test" FORCE)
set(SDL2MIXER_MIDI_FLUIDSYNTH OFF CACHE BOOL "Disable SDL2MIXER_MIDI_FLUIDSYNTH in SDL2_mixer" FORCE)
set(SDL2MIXER_WAVPACK OFF CACHE BOOL "Disable building SDL2MIXER_WAVPACK" FORCE)


FetchContent_MakeAvailable(SDL2_mixer)
#------------------------------------------------------------------------------
#------------------------------------------------------------------------------

#------------------------------------------------------------------------------
# Clang-Tidy
#------------------------------------------------------------------------------
if(ENABLE_CLANG_TIDY)
    message(STATUS "clang-tidy checks enabled")
    if(CMAKE_CXX_COMPILER_ID STREQUAL "Clang")
        # Adjust the checks, warning settings, etc. as you prefer:
        set(CMAKE_CXX_CLANG_TIDY    
            "clang-tidy"
        )
        message(STATUS "clang-tidy set:  ${CMAKE_CXX_CLANG_TIDY}")
    endif()
endif()
#------------------------------------------------------------------------------
#------------------------------------------------------------------------------

#------------------------------------------------------------------------------
# Corrosion
#------------------------------------------------------------------------------
FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.5
)
FetchContent_MakeAvailable(Corrosion)

set(RUST_BRIDGE_MANIFEST "${CMAKE_CURRENT_SOURCE_DIR}/rust_sdl_bridge/Cargo.toml")
corrosion_import_crate(MANIFEST_PATH ${RUST_BRIDGE_MANIFEST})
#------------------------------------------------------------------------------
#------------------------------------------------------------------------------

#------------------------------------------------------------------------------
# Source files
#------------------------------------------------------------------------------
set(SOURCE_FILES
src/rust_sdl_bridge.h
src/ANIMATION.cpp
src/GRAPHICS_Sprites_Draw.cpp
src/LOOP_Editor_Stages_Creator_Functions.cpp
src/LOOP_Stagefile_Load.cpp
src/STAGE_Draw.cpp
src/AUDIO.cpp
src/GRAPHICS_Sprites_Move.cpp
src/LOOP_Editor_Stages_Enemies.cpp
src/LOOP_Story.cpp
src/STAGE_Interface_Draw.cpp
src/CONTENT_Gametype_Load.cpp
src/GRAPHICS_Tiles_Animations.cpp
src/LOOP_Editor_Stages_Interface.cpp
src/PLAYER.cpp
src/SYSTEM_Debug_Infos.cpp
src/CONTENT_Stages_Load_Write.cpp
src/GRAPHICS_Tiles_Define.cpp
src/LOOP_Editor_Stages_Set_Elements.cpp
src/PLAYER_Collision_NonSolids.cpp
src/SYSTEM_Draw_Text.cpp
src/ENEMIES.cpp
src/GRAPHICS_Tiles_Draw.cpp
src/LOOP_Editor_Stages_Sprites.cpp
src/PLAYER_Collision_Solids.cpp
src/SYSTEM_Input_General.cpp
src/ENEMIES_Collision.cpp
src/GRAPHICS_Tiles_Elements.cpp
src/LOOP_Editor_Stages_Stage_Settings.cpp
src/PLAYER_Collision_Solids_Specials.cpp
src/SYSTEM_SDL_Textures.cpp
src/ENEMIES_Draw.cpp
src/GRAPHICS_Tiles_Switch_Tile.cpp
src/LOOP_Editor_Tiles.cpp
src/PLAYER_Collision_Sprites.cpp
src/SYSTEM_SDL_init.cpp
src/ENEMIES_Move.cpp
src/INTERFACE_Draw.cpp
src/LOOP_Gameloop.cpp
src/PLAYER_Draw.cpp
src/SYSTEM_SDL_input.cpp
src/GAME_ENVIRONMENT.cpp
src/LOOP_Credits.cpp
src/LOOP_Gameloop_Splashscreens.cpp
src/PLAYER_Jump.cpp
src/SYSTEM_Update_Screen.cpp
src/GRAPHICS_Sprites.cpp
src/LOOP_Editor_Stages.cpp
src/LOOP_Menu.cpp
src/PLAYER_Killed.cpp
src/SYSTEM_Vector_Operations.cpp
src/GRAPHICS_Sprites_Bullets.cpp
src/LOOP_Editor_Stages_Creator.cpp
src/LOOP_Options.cpp
src/PLAYER_Run.cpp
src/main.cpp
)
#------------------------------------------------------------------------------
#------------------------------------------------------------------------------
add_executable(openggs ${SOURCE_FILES})

target_include_directories(openggs PRIVATE
  "${SDL2_SOURCE_DIR}/include"
  "${SDL2_image_SOURCE_DIR}/include"
  "${SDL2_mixer_SOURCE_DIR}/include"
)

target_link_libraries(openggs PRIVATE 
    SDL2::SDL2
    SDL2_image::SDL2_image
    SDL2_mixer::SDL2_mixer
    rust_sdl_bridge)

add_dependencies(openggs rust_sdl_bridge)

add_custom_command(TARGET openggs POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        $<TARGET_FILE:SDL2::SDL2>
        $<TARGET_FILE:SDL2_image::SDL2_image>
        $<TARGET_FILE:SDL2_mixer::SDL2_mixer>
        $<TARGET_FILE_DIR:openggs>
    COMMAND ${CMAKE_COMMAND} -E copy_directory
        ${CMAKE_SOURCE_DIR}/base
        $<TARGET_FILE_DIR:openggs>/base
)


#------------------------------------------------------------------------------
# Clang-Format
#------------------------------------------------------------------------------
find_program(CLANG_FORMAT_BIN clang-format-18)
if (CLANG_FORMAT_BIN)
    message(STATUS "Found clang-format at ${CLANG_FORMAT_BIN}")

    # Gather all your source files. For example:
    file(GLOB_RECURSE ALL_SOURCE_FILES 
        "${CMAKE_SOURCE_DIR}/src/*.cpp"
        "${CMAKE_SOURCE_DIR}/src/*.h"
    )

    # Custom target: "format"
    add_custom_target(format
        COMMAND ${CLANG_FORMAT_BIN} -i ${ALL_SOURCE_FILES}
        COMMENT "Running clang-format on source files"
    )
endif()
#------------------------------------------------------------------------------
#------------------------------------------------------------------------------

if(WIN32)
    target_link_libraries(openggs
        PRIVATE
        SDL2::SDL2main
        ntdll
        ws2_32
        userenv
        advapi32
        shell32
    )
endif()
#------------------------------------------------------------------------------
#------------------------------------------------------------------------------
