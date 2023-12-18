# One threaded GPU benchmark

Runs a set of ray/triangle intersections on the GPU and CPU (262144 intersections by default) on **only one** thread of the CPU and GPU.

This is a very practical benchmark /s

On the CPU this benchmark is implemented in Rust. It uses [rust-gpu](https://github.com/EmbarkStudios/rust-gpu) to run in a compute shader on the GPU. It also runs [wgsl](https://www.w3.org/TR/WGSL/) and [slang](https://github.com/shader-slang/slang) versions of the shader.

The slang version is precompiled to SPIR-V, to manually compile make sure the env var for the `slanc` binary is setup and use `--compile-slang`.

The output of the CPU and GPU version may not match exactly on all GPUs.