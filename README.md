# rusty-boy
[![Build Status](https://travis-ci.org/MarkMcCaskey/rusty-boy.svg?branch=master)](https://travis-ci.org/MarkMcCaskey/rusty-boy)

A Gameboy emulator and related tools in Rust (with possible rendering also in C).

Making this live on twitch.tv/maoeurk


*Note*: This is under active development and is currently not in a
usable state.  See the bottom of this page for information about 
progress toward the first milestone.

Feel free to submit issues and pull requests.

## Building

First install `libsdl2-dev`.  If you're new to Rust, install `rustup`
to install `rustc` and `cargo`.

Then just run:

```
cd rusty-boy
cargo install
```

and you should be up and running.

## Version 0.1 milestone
- [ ] cpu
  - [x] opcodes
  - [x] dispatch
  - [ ] interrupts
  - [x] special registers
- [ ] graphics
  - [ ] background
  - [ ] sprites
  - [ ] window
- [x] controller input
- [ ] working sound
- [x] interactive debugger
  - [x] user interface
  - [x] breakpoints, watch points
  - [x] Basic usability (history, error handling)
  - [x] parsing of proper debug "language"
- [ ] run ROM-only game correctly
