cmake_minimum_required(VERSION 3.5)
project(rusty-boy)

set(CMAKE_C_STANDARD 99)
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${PROJECT_SOURCE_DIR}/cmake")
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")

find_package(SDL2 REQUIRED)
find_package(SDL2_Image REQUIRED)
find_package(SDL2_ttf REQUIRED)
find_package(SDL2_Mixer REQUIRED)

set(SOURCE_FILES src/C/graphics.c)
add_executable(${CMAKE_PROJECT_NAME} ${SOURCE_FILES})
include_directories(${SDL2_INCLUDE_DIR}
        ${SDL2_IMAGE_INCLUDE_DIR}
        ${SDL2_TTF_INCLUDE_DIR}
        ${SDL2_MIXER_INCLUDE_DIR})
target_link_libraries(${CMAKE_PROJECT_NAME} ${SDL2_LIBRARY}
        ${SDL2_IMAGE_LIBRARIES}
        ${SDL2_TTF_LIBRARIES}
        ${SDL2_MIXER_LIBRARIES})
