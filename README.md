# Enoki Schedulers

This repository includes the code for the Enoki schedulers, and some of the benchmarking code.

At this moment, only the the weighted fair queuing scheduler and the perf latency benchmark are documented. We plan to add scripts and documentation for the other schedulers and benchmarks in the future.

### Scheduler Dependencies
Install Rust using `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`.

Switch to the nightly toolchain using `rustup default nightly`.

This code was tested using a specific version of nightly and may not work with newer versions of the compiler. Install the tested version of rust using `rustup install nightly-2022-08-21`. The code may build using other versions, but if there are issues, try to use this version. Switch to this version using `rustup override set nightly-2022-08-21`.

### Compiling a scheduler
We only discuss how to build and install the weighted fair queuing scheduler, but the other schedulers follow a similar pattern.

Run `rustup component add rust-src`.

In the `wfq` directory, run `make`. The first time, this will take a few minutes.

### Loading a scheduler
Run `sudo mount -t ghost ghost /sys/fs/ghost/` to install the management file system used for setup.

Run `sudo insmod kernel/enoki_wfq.ko`.

### Unloading a scheduler
Run `sudo rmmod enoki_wfq`.

### Install perf
Before the perf latency benchmark can be run, you must install perf. Because the kernel is custom, you must install perf from the kernel source.

In the `enoki-kernel` directory under the master repository, run `sudo make -C tools/perf install`.

### Running the perf latency benchmark
With the scheduler loaded, move to the `perf_test` directory.

Run `make` in the `perf_test` directory.

Run `sudo ./enoki_test` to run the perf latency test on the loaded Enoki scheduler.

To run the test on CFS, use `sudo ./cfs_test`.
