# rusty-boy
[![Build Status](https://travis-ci.org/MarkMcCaskey/rusty-boy.svg?branch=master)](https://travis-ci.org/MarkMcCaskey/rusty-boy)

A Gameboy emulator and related tools in Rust (with possible rendering also in C).

Making this live on twitch.tv/maoeurk


*Note*: This is under active development and is currently not in a
usable state.  For an incomplete overview of work left to be done, see
`src/todo.org`.

Feel free to submit issues and pull requests.

## Building

First install `libsdl2-dev`.  If you're new to Rust, install `rustup`
to install `rustc` and `cargo`.

Note: There seems to be problems building this with the `stable`
branch of `rustc`, change your default to `nightly` before building by
running `rustup default nightly`.

Then just run:

```
cd rusty-boy
cargo install
```

and you should be up and running.
