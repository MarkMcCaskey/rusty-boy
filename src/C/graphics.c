#include <SDL2/SDL.h>
#include <stdio.h>
#include <stdbool.h>
#include "graphics.h"

void logSDLError(const char* msg) {
    fprintf(stderr, "error: %s %s", msg, SDL_GetError());
}

// After creating a window please remember to free it, returns bool which signifies success or failure
_Bool create_window(Window* win, char* window_name, int width, int height, SDL_WindowFlags win_flags, short ren_index, SDL_RendererFlags ren_flags) {
    if (SDL_Init(SDL_INIT_EVERYTHING) != 0) {
        logSDLError("Unable to Initialise SDL");
        return false;
    }

    win = (Window *) malloc(sizeof(Window));
    if (win == NULL) {
        logSDLError("Unable to allocate Window");
        return false;
    }

    // Initialise the dynamically allocated window
    win->name = window_name;
    win->width = width;
    win->height = height;
    win->win = SDL_CreateWindow(window_name, SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED,
                                    width, height, win_flags);
    if (win->win == NULL) {
        logSDLError("Unable to create SDL_Window");
        return false;
    }
    win->ren = SDL_CreateRenderer(win->win, ren_index, ren_flags);
    if (win->ren == NULL) {
        logSDLError("Unable to create SDL_Renderer");
        return false;
    }

    return true;
}

// Frees window then returns a boolean which signifies success or failure
_Bool free_window(Window *window) {

    // Free the SDL members first
    SDL_DestroyWindow(window->win);
    if (window->win != NULL) {
        logSDLError("Unable to free SDL_Window");
        return false;
    }
    SDL_DestroyRenderer(window->ren);
    if (window->ren != NULL) {
        logSDLError("Unable to free SDL_Renderer");
        return false;
    }

    // Free the rest of the class
    free(window);
    if (window != NULL) {
        logSDLError("Unable to free Window");
        return false;
    }

    return true;
}

// Draws a pixel, what else did you expect
void draw_pixel(Window *window, int x, int y, SDL_Color color, uint8_t alpha) {
    SDL_Rect draw_dst;
    draw_dst.x = x;
    draw_dst.y = y;
    draw_dst.w = PIXEL_SIZE;
    draw_dst.h = PIXEL_SIZE;

    SDL_SetRenderDrawColor(window->ren, color.r, color.g, color.b, alpha);
    SDL_RenderDrawRect(window->ren, &draw_dst);
}
