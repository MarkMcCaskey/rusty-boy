#ifndef RUSTY_BOY_GRAPHICS_H
#define RUSTY_BOY_GRAPHICS_H

#define TILE_SIZE 8
#define PIXEL_SIZE 1

#include "linked_list.h"

// Little wrapper for SDL to make it into a nice neat bun
typedef struct Window {
    // Name of the window
    char* name;

    // In terms of tiles
    int width;
    int height;

    // Window object
    SDL_WindowFlags win_flags;
    SDL_Window *win;

    // Renderer object
    SDL_RendererFlags ren_flags;
    SDL_Renderer *ren;

    // The sprites Maximum 10
    List sprite_list;
    int index; // Default initialised to 0
} Window;

_Bool create_window(Window* win, char* window_name, int width, int height, SDL_WindowFlags win_flags, short ren_index, SDL_RendererFlags ren_flags);
_Bool free_window(Window *window);
void draw_pixel(Window *window, int x, int y, SDL_Color color, uint8_t alpha);

#endif //RUSTY_BOY_GRAPHICS_H
