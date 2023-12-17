# One threaded GPU benchmark

Runs a set of ray/triangle intersections on the GPU and CPU (262144 intersections by default) on **only one** thread of the CPU and GPU.

This is a very practical benchmark /s

The benchmark is written in Rust and uses [rust-gpu](https://github.com/EmbarkStudios/rust-gpu) to run in a compute shader on the GPU.

The output of the CPU and GPU version may not match exactly on all GPUs. (They did match on an RTX3060, but did not on an Intel UHD 630 IGP)