
# Hello-AI

This repo is a single rust binary which exercises the library [`floneum`](https://github.com/floneum/floneum)
to perform important tasks that will eventually go into a more well-thought-out game.

# Compiling

To transform the source code into a `.exe` program, run

```
cargo build --release
```


# Running

After compiling, run `./target/release/hello-ai-game[.exe]`

You can directly compile & run in one step via `cargo run --release`


# Miscellaneous To-Dos

 - [ ] `floneum/kalosm` download models automatically to `???` - is there a mechanism to control where these files go so when we build an installer we can ship 1 big component instead of downloading the world?
 - [ ] What magic needs to be invoked for `floneum/kalosm` GPU support? This will allow big hardware to run the game fast!
 - [ ] Of the models or model formats (`gguf`, `ggml`, `safetensors` et al) integrated into `floneum/kalosm`, which models would be best for various game mechanics? What are the hardware-requirement / performance trade-offs?
 - [ ] There exist many graphics libraries, and rust has bindings to a lot of them. Which graphics system makes sense for a videogame, and do we want to focus local-only or play with a web-based design to serve graphics as HTML instead of native buttons? (https://github.com/rust-unofficial/awesome-rust?tab=readme-ov-file#gui)





