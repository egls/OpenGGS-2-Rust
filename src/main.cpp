#include "globals.h"
#include <filesystem>
#include <string>

// needed because SDL does not create stdout and stderr anymore...
FILE* my_stdout = freopen("my_stdout.txt", "wb" /*or "wt"*/, stdout);
FILE* my_stderr = freopen("my_stderr.txt", "wb" /*or "wt"*/, stderr);

bool QuitProgram;
Config_Type Config;

void init();   //Starts up SDL and creates window
void close();  //Frees media and shuts down SDL
void iniSetup();

// ##############################################
// ##############################################
// ##############################################

#include "rust_sdl_bridge.h"

static void sync_rust_runtime_state() {
  RustBridgeConfig rust_config;
  rust_config.Resolution = GV.Resolution;
  rust_config.Volume_Music = VolumePercentage_Music;
  rust_config.Volume_Sound = VolumePercentage_Sound;

  rust_bridge_sync_runtime_state((unsigned char)(QuitProgram ? 1 : 0),
                                 rust_config);
}

static std::string configure_asset_root_and_working_dir() {
  namespace fs = std::filesystem;
  std::error_code ec;
  fs::path cwd = fs::current_path(ec);
  if (ec) {
    cwd.clear();
  }

  fs::path exe_dir;
  char* sdl_base_path = SDL_GetBasePath();
  if (sdl_base_path != NULL) {
    exe_dir = fs::path(sdl_base_path);
    SDL_free(sdl_base_path);
  }

  fs::path candidate = cwd / "base";
  if (!cwd.empty() && fs::exists(candidate, ec) && !ec) {
    rust_bridge_set_asset_root(candidate.string().c_str());
    return candidate.string();
  }

  candidate = exe_dir / "base";
  if (!exe_dir.empty() && fs::exists(candidate, ec) && !ec) {
    fs::current_path(exe_dir, ec);
    rust_bridge_set_asset_root(candidate.string().c_str());
    return candidate.string();
  }

  candidate = exe_dir.parent_path() / "base";
  if (!exe_dir.empty() && fs::exists(candidate, ec) && !ec) {
    fs::current_path(exe_dir.parent_path(), ec);
    rust_bridge_set_asset_root(candidate.string().c_str());
    return candidate.string();
  }

  rust_bridge_set_asset_root("base");
  return "base";
}

int main(int argc, char* args[]) {
  // Instead of SDL_Init(...), do:
  if (init_sdl2_bridge() < 0) {
    std::cerr << "Failed to initialize SDL (via Rust)\n";
    return 1;
  }
  std::cout << "SDL Initialized by Rust!\n"; // output in std_out.txt

  configure_asset_root_and_working_dir();

  GAME_ENVIRONMENT_Define();

  init();             // Start up SDL and create window
  srand(time(NULL));  // initialize random seed
  defineAngles();

  Option_GameType_Load();
  GAMETYPE_Load();

  QuitProgram = false;
  sync_rust_runtime_state();

  LOOP_Menu();
  sync_rust_runtime_state();

  close();  //Free resources and close SDL objects
  quit_sdl2_bridge();
  std::cout << "SDL Quit (via Rust)\n";
  return 0; 
}

// ##############################################
// ##############################################
// ##############################################

void close() {

  //Destroy window
  SDL_DestroyRenderer(gRenderer);
  SDL_DestroyWindow(gWindow);
  // SDL_JoystickClose( GameController );
  //  GameController = NULL;

  gWindow = NULL;
  gRenderer = NULL;
  // SDL shutdown is owned by rust_sdl_bridge.
}

// ##############################################
// ##############################################
// ##############################################
