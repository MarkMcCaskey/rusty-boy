//
// Created by JULIA BALAJAN on 12/01/2017.
//

#include "linked_list.h"
#include <stdio.h>

// Constructor for a linked list
// Creates one node
List new_list(SDL_Texture *data) {
    // Creates node
    Node *new_node = (Node *) malloc(sizeof(Node));

    // Initialise node
    new_node->data = data;
    new_node->previous = NULL;
    new_node->next = NULL;

    // Create new list
    List new_list;
    new_list.front = new_node;
    new_list.back = new_node;

    // Set list size
    new_list.size = 1;

    // Return list
    return new_list;
}

// Constructor for a linked list
// Creates empty list
List new_empty_list() {
    // Create new list
    List new_list;
    new_list.front = NULL;
    new_list.back = NULL;

    // Set list size
    new_list.size = 0;

    // Return list
    return new_list;
}

// Constructor for a linked list
// Creates empty list of desired length
List new_empty_list_n(int n) {
    // Create new list
    List new_list;

    Node *nodes[n];
    for (int i = 0; i < n; ++i) {
        nodes[i] = (Node *) malloc(sizeof(Node));
        nodes[i]->data = NULL;
    }

    Node *previous_node = NULL;
    for (int i = 0; i < n; ++i) {
        nodes[i]->previous = previous_node;
        previous_node = nodes[i];
    }

    Node *next_node = NULL;
    for (int i = n - 1; i >= 0; --i) {
        nodes[i]->next = next_node;
        next_node = nodes[i];
    }

    new_list.front = nodes[0];
    new_list.back = nodes[n - 1];
    new_list.size = n;

    return new_list;
}

// Destructor for a linked list
void destroy_list(List *list) {
    // Iterate from the back to the front deleting Nodes
    Node *iterator_1 = list->back;
    Node *iterator_2 = list->back->previous;
    while (iterator_2 != NULL) {
        free(iterator_1);
        iterator_1 = iterator_2;
        iterator_2 = iterator_2->previous;
    }
    free(iterator_1);

    // Set list front and back to NULL
    list->front = NULL;
    list->back = NULL;

    // Update list size
    list->size = 0;
}

// Only used to push data to the back of a list
void push_back(List *list, SDL_Texture *data) {
    // Create the new node
    Node *new_node = (Node *) malloc(sizeof(Node));

    // Link the new node into the list
    list->back->next = new_node;
    new_node->next = NULL;
    new_node->previous = list->back->next;

    // Sets new node as the last element
    list->back = new_node;

    // Update list size
    ++list->size;
}

// Only used to push data to the front of a list
void push_front(List *list, SDL_Texture *data) {
    // Create the new node
    Node *new_node = (Node *) malloc(sizeof(Node));

    // Link the new node into the list
    list->front->previous = new_node;
    new_node->previous = NULL;
    new_node->next = list->front->previous;

    // Sets new node as the first element
    list->front = new_node;

    // Update list size
    ++list->size;
}

// Insert a node between two nodes
void insert(List *list, Node *previous_node, Node *next_node, SDL_Texture *data) {
    // Create the new node
    Node *new_node = (Node *) malloc(sizeof(Node));

    // Link the new node into the list
    new_node->data = data;

    new_node->previous = previous_node;
    new_node->next = next_node;

    previous_node->next = new_node;
    next_node->previous = new_node;

    // Update list size
    ++list->size;
}

// Pops back element
void pop_back(List *list) {
    // Cache value of previous node
    Node *previous_node = list->back->previous;

    // Free the node
    free(list->back);

    // Re-link the list
    previous_node->next = NULL;

    // Update the list size
    --list->size;
}

// Pops front element
void pop_front(List *list) {
    // Cache value of next node
    Node *next_node = list->front->next;

    // Free the node
    free(list->front);

    // Re-link list
    next_node->previous = NULL;

    // Update the list size
    --list->size;
}

// Deletes node
void delete(List *list, Node *node) {
    // Cache values of previous and next nodes
    Node *previous_node = node->previous;
    Node *next_node = node->next;

    // Free the node
    free(node);

    // Re-link the list
    previous_node->next = next_node;
    next_node->previous = previous_node;

    // Update list size
    --list->size;
}

// Dereference node at index storing it in data
// Returns boolean indicating success or failure
bool dereference_index(SDL_Texture **data, List *list, int index) {
    // Checks to see if index is out of range
    if (index < 0 || index > list->size - 1) {
        fprintf(stderr, "Dereferenced value out of list range");
        return false;
    }

    // Iterates to index
    Node *iterator = list->front;
    for (int i = 0; i < index; ++i)
        iterator = iterator->next;

    // Sets data to value
    data = &(iterator->data);
    return true;
}
