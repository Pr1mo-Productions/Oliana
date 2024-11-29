
# Oliana

This repo contains the game runtime for Oliana! Non-techie details on the [Landing Page](https://BM-Enterprises.github.io/Oliana/) (content set in `docs/index.html`),
techie details below.

# Game Design Ideas



# Source Code

As with most rust programs, execution begins at `src/main.rs` `main()`.

# Dependencies

__Required__

 - [Rust](https://rustup.rs/)
 - [onnxruntime](https://onnxruntime.ai/)
    - Arch: `sudo pacman -S onnxruntime-opt openmpi`
 - [ollama](https://ollama.com/)
    - Arch: `sudo pacman -S ollama`
 - [libtorch-cuda](https://pytorch.org/)
    - Arch: `yay -S libtorch-cuda`
    - This gives diffusers CUDA capabilities, which we're engineering on the assumption exist b/c it's a well-tested path to executing models quickly.


__Optional__

 - [cuDNN](https://developer.nvidia.com/cuDNN)
    - Build with `cargo build --features cuda --release` to link against cuDNN and expose GPU processing capabilities of `floneum/kalosm`


# Compiling

To transform the source code into a `.exe` program, run

```
cargo build --release
```


# Running

After compiling, run `./target/release/Oliana[.exe]`

You can directly compile & run in one step via `cargo run --release`

# Design Decisions Log

 - We're using [ORT](https://ort.pyke.io/) `2.0` for local in-process LLM inferencing and [Ollama](https://ollama.com/) for local out-of-process LLM inferencing.
    - Design reason for ORT: ONNX looks like a solid bet for long-term model ingestion, management, and is designed to support text and image-based AI models.
    - Design reason for Ollama: Ollama just makes throwing pre-existing models at things easy; I see Ollama as a gateway to testing models which we will then manually convert to ONNX files for the game.
 - We're using [Bevy](https://bevyengine.org/) as our graphics framework; it has more capabilities than we will need and is cross-platform, all we will need to do is learn the engine and map our deisgns into Bevy's structures.
 - We're using Github Pages as our [Landing Page](https://BM-Enterprises.github.io/Oliana/) (content set in `docs/index.html`) because it's free and easy to setup.



# Miscellaneous To-Dos

 - [ ] Which models would be best for various game mechanics? What are the hardware-requirement / performance trade-offs?
 - [x] There exist many graphics libraries, and rust has bindings to a lot of them. Which graphics system makes sense for a videogame, and do we want to focus local-only or play with a web-based design to serve graphics as HTML instead of native buttons? (https://github.com/rust-unofficial/awesome-rust?tabq=readme-ov-file#gui)
    - Jeffrey decided on using [`bevy`](https://bevyengine.org/) because it's got capabilities out the wazoo; definitely a larger game engine than what we need but it's cross-platform and has tons of hooks to customize everything.
 - [ ] for long-term or plot-related LLM memory - what existing tools make sense to use? (https://github.com/jondot/awesome-rust-llm?tab=readme-ov-file#llm-memory)

 - [ ] When loading `*.onnx` files w/ external tensor weights (ie in a folder w/ `` files containing actual weights), why doesn't `ORT` load everything? Relevant - https://github.com/pykeio/ort/issues/39
    - We see error `op_kernel_context.h:42 const T* onnxruntime::OpKernelContext::Input(int) const [with T = onnxruntime::Tensor] Missing Input: onnx::Neg_58`, where `onnx::Neg_58` is a set of weights referenced by the original `*.onnx` file.
    - This is likely our `utils/oneshot-convert-hf-model-to-onnx.py` missing some step yielding an incomplete result.


# Miscellaneous Research

`Wuerstchen` sounds like a nice image-generation model;... for people with 16gb+ vram Nvidia cards. As I only have a 12gb 3080ti and Intel's a770 doesn't have anything besides LLMs ported to it,
a more resource-efficient image generator should be found. This thing looks promising!
 - https://github.com/Gadersd/stable-diffusion-burn/tree/main?tab=readme-ov-file#stable-diffusion-burn

This looks like a good beginning for vector databases & long-term information storage & retrieval LLM-style:

 - https://github.com/Mintplex-Labs/anything-llm

This is a neat capability we should bolt on after getting basic AI image-gen done!

 - https://github.com/SET001/bevy_scroller

Speaking of in-process image gen, this looks nice and simple!

 - https://github.com/RobertBeckebans/AI_text2img_diffusers-rs


# Utility scripts


## `oneshot-convert-hf-model-to-onnx.py`

`utils/oneshot-convert-hf-model-to-onnx.py` exists to convert a hugginface repo to `*.onnx` files; it does not need any dependencies besides `git`, `python`, and the ability for `git-lfs` to be installed.

Usage:

```bash
python utils/oneshot-convert-hf-model-to-onnx.py https://huggingface.co/Qwen/Qwen2.5-7B-Instruct

```





