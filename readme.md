
# Hello-AI

This repo is a single rust binary which exercises the library [`floneum`](https://github.com/floneum/floneum)
to perform important tasks that will eventually go into a more well-thought-out game.

[Landing Page](https://jeffrey-p-mcateer.github.io/hello-ai-game/) (content set in `docs/index.html`)

# Game Design Ideas



# Source Code

As with most rust programs, execution begins at `src/main.rs` `main()`.

# Dependencies

__Required__

 - [Rust](https://rustup.rs/)
 - [onnxruntime](https://onnxruntime.ai/)
    - Arch: `sudo pacman -S onnxruntime-opt openmpi`

__Optional__

 - [cuDNN](https://developer.nvidia.com/cuDNN)
    - Build with `cargo build --features cuda --release` to link against cuDNN and expose GPU processing capabilities of `floneum/kalosm`


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
 - [x] There exist many graphics libraries, and rust has bindings to a lot of them. Which graphics system makes sense for a videogame, and do we want to focus local-only or play with a web-based design to serve graphics as HTML instead of native buttons? (https://github.com/rust-unofficial/awesome-rust?tabq=readme-ov-file#gui)
    - Jeffrey decided on using [`bevy`](https://bevyengine.org/) because it's got capabilities out the wazoo; definitely a larger game engine than what we need but it's cross-platform and has tons of hooks to customize everything.
 - [ ] for long-term or plot-related LLM memory - what existing tools make sense to use? (https://github.com/jondot/awesome-rust-llm?tab=readme-ov-file#llm-memory)

# Miscellaneous Research

`Wuerstchen` sounds like a nice image-generation model;... for people with 16gb+ vram Nvidia cards. As I only have a 12gb 3080ti and Intel's a770 doesn't have anything besides LLMs ported to it,
a more resource-efficient image generator should be found. This thing looks promising!
 - https://github.com/Gadersd/stable-diffusion-burn/tree/main?tab=readme-ov-file#stable-diffusion-burn

# Utility scripts


## `oneshot-convert-hf-model-to-onnx.py`

`utils/oneshot-convert-hf-model-to-onnx.py` exists to convert a hugginface repo to `*.onnx` files; it does not need any dependencies besides `git`, `python`, and the ability for `git-lfs` to be installed.

Usage:

```bash
python utils/oneshot-convert-hf-model-to-onnx.py https://huggingface.co/Qwen/Qwen2.5-7B-Instruct

```





