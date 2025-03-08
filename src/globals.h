
#include <SDL.h>
#include <SDL_image.h>
#include <SDL_mixer.h>

#include <dirent.h>  // NEEDED TO CHECK EXISTENCE OF DIRECTORIES
#include <stdio.h>   // include ability to read/write files
#include <stdlib.h>  // srand
#include <string.h>
#include <time.h>
#include <cmath>
#include <iostream>

#include "AUDIO.h"
#include "CONTENT_Enemies.h"
#include "CONTENT_Player.h"
#include "CONTENT_Stages.h"
#include "CONTENT_Tiles_and_Sprites.h"
#include "GAME_ENVIRONMENT.h"
#include "INTERFACE.h"
#include "LOOPS.h"
#include "SYSTEM.h"
#include "SYSTEM_SDL.h"
#include "SYSTEM_SDL_Input.h"

void Animation();
void drawGround(int Colour);

// ##############################################
// ##############################################
// ##############################################
