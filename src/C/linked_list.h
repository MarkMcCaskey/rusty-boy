//
// Created by JULIA BALAJAN on 12/01/2017.
//

#ifndef RUSTY_BOY_LINKED_LIST_H
#define RUSTY_BOY_LINKED_LIST_H

typedef struct Node {
    Node *next;
    Node *previous;
    SDL_Texture *data;
} Node;

typedef struct List {
    Node *front;
    Node *back;
    int size;
} List;

List new_list(SDL_Texture *data);
List new_empty_list();
List new_empty_list_n(size_t n);
void destroy_list(List *list);
void push_back(List *list, SDL_Texture *data);
void push_front(List *list, SDL_Texture *data);
void insert(List *list, Node *previous_node, Node *next_node, SDL_Texture *data);
void pop_back(List *list);
void pop_front(List *list);
void delete(List *list, Node *node);
bool dereference_index(SDL_Texture **data, List *list, size_t index);

#endif //RUSTY_BOY_LINKED_LIST_H
