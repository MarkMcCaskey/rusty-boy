# rusty-boy
[![Build Status](https://travis-ci.org/MarkMcCaskey/rusty-boy.svg?branch=master)](https://travis-ci.org/MarkMcCaskey/rusty-boy)

A Gameboy emulator and related tools in Rust

It's currently in a very unstable state.

Made live on twitch.tv/maoeurk
(Unfortunately I'm not currently streaming, but I'd like to resume in the near future)

## WARNING

V0.1.1 introduces a lot of instability and has many regressions

Don't expect it to work outside of a development context at this stage.

Contributions welcome!

## About

Project done for fun and learning about Rust and project management.

Memory visualization inspired by [ICU64 / Frodo Redpill v0.1](https://icu64.blogspot.com/2009/09/first-public-release-of-icu64frodo.html)

V0.1.0: image of memory visualization of Tetris.
![tetris v0.1.0](images/tetris.0.1.0.png)

V0.1.0: Game, _Popup_, running with the ncurses debugger.
![popup debugger v0.1.0](images/popup-debugger.0.1.0.png)

## State of the project

Things came up and I stopped developing this and streaming.  The primary reason being that the change from 0.1.0 to 0.2.0 was too large and became unmanageable.

Current goals:

- [ ] Reduce scope of project

- [ ] Rewrite rendering and visualization to be GPU based (Vulkan or OpenGL)

- [ ] Finish rendering (sprite flipping, textboxes)

- [ ] Rewrite sound (properly and with visualizations)

- [ ] Refactor CPU, memory management, and IO (basically the entire project)

- [ ] Optimize CPU execution (high level JIT? no machine code generation, just recompiled to a more performant bytecode) if it's a performance bottleneck

- [ ] Fix usability bugs (rebindable keys, some kind of more complete controller support)

- [ ] Fix regressions introduced in 0.1.1 (Tetris crashes now ;_;)


Non goals:

- Fancy compilers, assemblers, and debug/dev tools (I'll add support for these in a decoupled way if I actually finish the tasks above)

- Full support for extra features (like the camera)

= Be a better product than existing emulators (I'm sure there are much nicer emulators for playing games.  Usability is not a primary goal at this time)

## Building

First install `libsdl2-dev`.  If you're new to Rust, install `rustup`
to install `rustc` and `cargo`.

You may need to install ncurses libraries to compile this project.
TODO: test and update this

Then just run:

```
cd rusty-boy
cargo install
```

and you should be up and running.

## Running

To run, run the following command:
```
cargo run --release -- /path/to/rom
```

To run with the TUI debugger, run:
```
cargo run --release --features=debugger -- /path/to/rom -d
```

To run with the debugger, run:
```
cargo run --release --features="debugger" -- /path/to/rom -d
```
