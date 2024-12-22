
# Oliana

This repo contains the game runtime for Oliana! Non-techie details on the [Landing Page](https://Pr1mo-Productions.github.io/Oliana/) (content set in `docs/index.html`),
techie details below.


Oliana is made up of several sub-programs. This design minimises damage when one component needs a large re-design, and gives us strong interfaces that will help scale things out as we prove sub-component capabilities.


## `Oliana-Lib`

**Goal:** For all behaviors common to other tools, place them here and include into the other programs.

**Status:** `ollama_lib` gives some utility functions such as:

 - `oliana_lib::files::get_cache_file(<file-name>)`
    - uses `dirs` to join file paths to a local app-specific folder (ie `%LocalAppData%\AppName\<file-name>` on windows, `~/.cache/AppName/<file-name>` on linux)
 - `oliana_lib::files::existinate(<local-file-path>, <url>)`
    - Downloads file if it does not exist, returning the file path

 - `oliana_lib::err::eloc!()`
    - Useful for adding line numbers to rust Error returns; we commonly use `-> Result<THE_TYPE_WE_WANT, Box<dyn std::error::Error>>` to avoid caring about detailed errors, but line numbers are nice to add to these!

## `Oliana-Images`

**Goal:** Build a stand-alone executable that can

1. Download all files it needs to some local cache folder
2. Execute a GPU-Accelerated text-to-image pipeline

**Status:** Success! When run like `oliana_images[.exe] --workdir /path/to/folder`, any newly-created `X.json` files are read and `X.png` is written back. If an error occurs, `X.txt` will contain a python stack-trace. Image model files are stored in `~/.cache/oliana_lib/Oliana-Images-hf_home` (linux, mac) or `%LOCALAPPDATA%\oliana_lib\Oliana-Images-hf_home` (windows)

**Dependencies**

 - Python `3.10+`
    - the program links against your system python and installs all libraries under `~/.cache/oliana_lib/Oliana-Images-site_packages` (linux, mac) or `%LOCALAPPDATA%\oliana_lib\Oliana-Images-site_packages` (windows)
 - CUDA
    - consult your operating system documentation for Nvidia drivers & Cuda userspace libraries.



## `Oliana-Text`

**Goal:** Build a stand-alone executable that can

1. Download all files it needs to some local cache folder
2. Execute a GPU-Accelerated context-question-answer pipeline

**Status:** The current implementation runs `microsoft/Phi-3.5-mini-instruct` on the GPU, but we don't control where model files are saved to. The library does respect `HF_HOME` though, so we can use another process to set this before running `oliana_text[.exe]` to control where model files are saved to.


```bash
cargo run --release --bin oliana_text
```

Requirements for running bare `oliana_text[.exe]`:

 - None! `\o/`


## `Oliana-Server`

**Goal:** Build a stand-alone webserver & library that allows bi-directional communication between a system without a GPU and a system WITH a GPU to run the following sub-tools:

 - `oliana_images[.exe]`
    - Given some text prompt, return in-progress images and the final image from a diffusion run on a GPU.
 - `oliana_text[.exe]`
    - Given some text prompt, return tokens as they are generated w/ a sentinel value to indicate the end at the final token.

**Stretch Goal:** Keep the same model files in-memory so clients don't have to pay start-up costs for each request to generate an image or text.

**Status:** We have a minimal server-client async RPC using `tarpc` + `serde` for binary transport over IPv6 and IPv4 TCP (some systems resolve `localhost` to `127.0.0.1`, others will resolve `localhost` to `::1/128`). We don't have a good client interface yet and the server doesn't interact with `Oliana-Images` or `Oliana-Text`.

All of the above decisions mean our server can hold a long-term, two-way communication channel that can pass primitive types around; probably the most complex type we will pass is the result of `Oliana-Images`, which we can standardize as a `Vec<u8>` holding `.png` bytes of a single frame.



```bash
# In terminal A
cargo run --release --bin oliana_server
# In terminal B
cargo run --release --bin oliana_client

```

## `Oliana-CLI`

**Goal:** Build a command-line tool capable of running the other tools to play `Oliana`-the-game in a command-line text-based aventure!

**Stretch Goal:** Also add capabilities to download the other tools off the github releases page or similar distribution channel; TODO think about packaging ideas & how updates will work

## `Oliana-GUI`

**Goal:** Build a GUI tool capable of running the other tools to play `Oliana`-the-game in a graphical text-based aventure!

**Stretch Goal:** Also add capabilities to download the other tools off the github releases page or similar distribution channel; TODO think about packaging ideas & how updates will work

[Tracking Issue 1](https://github.com/Pr1mo-Productions/Oliana/issues/1)


# Misc Notes

`Oliana-CLI[.exe]` and `Oliana-GUI[.exe]` are going to share a lot of logic; we may either place that in `Oliana-Lib` or we may create a shared `Oliana-GameLogic` library to hold it.

`torch-sys` is a pain to build reliably; the easest approach is to use the system-provided copy of torch by running:

 - `yay -S libtorch-cxx11abi-cpu libtorch-cxx11abi-cuda`
    - Installs the libraries
 - `sudo ln -s /opt/libtorch-cuda/lib/libtorch.so /usr/lib/libtorch.so`
 - `sudo ln -s /opt/libtorch-cuda/lib/libc10.so /lib/libc10.so`
 - `sudo ln -s /opt/libtorch-cuda/lib/libtorch_cpu.so /lib/libtorch_cpu.so`
 - `sudo ln -s /opt/libtorch-cuda/lib/libtorch_cuda.so /lib/libtorch_cuda.so`
 - `sudo ln -s /opt/libtorch-cuda/lib/libgomp-98b21ff3.so.1 /lib/libgomp-98b21ff3.so.1`
    - Aids poorly-written linkers to find their libraries `-_-`

Throw the following in `/usr/lib/pkgconfig/torch.pc`:

```
libdir=/opt/libtorch-cuda/lib
includedir=/opt/libtorch-cuda/include

Name: torch
Description: Torch Library
Version: 11.0
Libs: -L${libdir} -ltorch_cuda -ltorch_cpu
Cflags: -I${includedir}
```


