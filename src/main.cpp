#include "globals.h"

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

int main(int argc, char* args[]) {
  // Instead of SDL_Init(...), do:
  if (init_sdl2_bridge() < 0) {
    std::cerr << "Failed to initialize SDL (via Rust)\n";
    return 1;
}

std::cout << "SDL Initialized by Rust!\n"; // output in std_out.txt

// ... do stuff ...

// Instead of SDL_Quit(), do:
quit_sdl2_bridge();
std::cout << "SDL Quit (via Rust)\n";
return 0;

/*   GAME_ENVIRONMENT_Define();

  init();             // Start up SDL and create window
  srand(time(NULL));  // initialize random seed
  defineAngles();

  Option_GameType_Load();
  GAMETYPE_Load();

  QuitProgram = false;

  LOOP_Menu();

  close();  //Free resources and close SDL
  return 0; */
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
  //Quit SDL subsystems
  IMG_Quit();
  SDL_Quit();
}

// ##############################################
// ##############################################
// ##############################################
