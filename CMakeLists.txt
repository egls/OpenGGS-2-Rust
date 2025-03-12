cmake_minimum_required(VERSION 3.14)
project(OpenGGS LANGUAGES CXX)

# Optionally set C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(FetchContent)

# Declare SDL2 
FetchContent_Declare(
  SDL2
  GIT_REPOSITORY https://github.com/libsdl-org/SDL.git
  GIT_TAG release-2.32.2
)

# Disable building SDL2 tests, if desired
set(SDL_TEST OFF CACHE BOOL "Do not build SDL2_test" FORCE)

# Optionally disable tests or set build options
set(SUPPORT_AVIF OFF CACHE BOOL "Disable AVIF support" FORCE)
set(SDL2IMAGE_AVIF OFF CACHE BOOL "Disable TIFF support in SDL2_image" FORCE)
set(SUPPORT_WEBP OFF CACHE BOOL "Disable WEBP support" FORCE)
set(SUPPORT_TIFF OFF CACHE BOOL "Disable TIFF support in SDL2_image" FORCE)
set(SDL2IMAGE_TIF OFF CACHE BOOL "Disable TIFF support in SDL2_image" FORCE)

set(IMAGE_TEST OFF CACHE BOOL "Disable building SDL2_image test" FORCE)
# --- Fetch & build SDL2_image ---
FetchContent_Declare(
  SDL2_image
  GIT_REPOSITORY https://github.com/libsdl-org/SDL_image.git
  GIT_TAG release-2.8.8
)

# Disable AVIF support (which uses dav1d)

# Before FetchContent_MakeAvailable(SDL2_mixer):
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
# --- Fetch & build SDL2_mixer ---
FetchContent_Declare(
  SDL2_mixer
  GIT_REPOSITORY https://github.com/libsdl-org/SDL_mixer.git
  GIT_TAG release-2.8.1
)


# 3) Fetch (clone) and build SDL2, SDL2_image, and SDL2_mixer
FetchContent_MakeAvailable(SDL2 SDL2_image SDL2_mixer)


if(CMAKE_CXX_COMPILER_ID STREQUAL "__Clang")
    # Adjust the checks, warning settings, etc. as you prefer:
    set(CMAKE_CXX_CLANG_TIDY 
        "clang-tidy"
    )
    message(STATUS "clang-tidy set:  ${CMAKE_CXX_CLANG_TIDY}")
endif()

set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${OpenGGS_SOURCE_DIR}/cmake")

if (CMAKE_SYSTEM_NAME STREQUAL "Linux")
    find_package(SDL2 REQUIRED)
    find_package(SDL2_image REQUIRED)
    find_package(SDL2_mixer REQUIRED)

    include_directories(${SDL2_INCLUDE_DIR}
                        ${SDL2_IMAGE_INCLUDE_DIR}
                        ${SDL2_MIXER_INCLUDE_DIR})
endif()


set(SOURCE_FILES
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

add_executable(openggs ${SOURCE_FILES})

if (CMAKE_SYSTEM_NAME STREQUAL "Linux")
target_link_libraries(openggs ${SDL2_LIBRARY}
                             ${SDL2_IMAGE_LIBRARIES}
                             ${SDL2_MIXER_LIBRARIES})
endif()

target_link_libraries(openggs
    PRIVATE
        SDL2::SDL2
        SDL2::SDL2main    # needed on Windows
        SDL2_image
        SDL2_mixer

)