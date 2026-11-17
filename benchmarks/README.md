# Benchmarks

How long an edit takes until its result is computed, on the CPU with one
thread, on the CPU with every core, and on the GPU.

```bash
cargo run -p oneiroi-bench --release
```

prints a table of every scenario at every size and compares it with the
baseline of the machine it runs on. `--quick` measures fewer sizes, `--only
<scenario>` a single scenario, `--backends cpu,gpu` only some backends.
`WGPU_BACKEND` picks the backend of the GPU, `--features dxc` gives DX12
its mesh shaders.

## Baselines

`results/` holds a baseline per machine, named after its operating system,
CPU, GPU and the backend of the GPU. A run is only compared with the
baseline of its own machine, and a time counts as changed only when it
changed by more than 5% and by more than the runs vary anyway.

```bash
cargo run -p oneiroi-bench --release -- --accept
```

makes the numbers of the run the new baseline and writes a table of it next
to it. Commit both: the diff then shows what got faster or slower.

## Counters

What the GPU executor does for an edit (which nodes run where, how many
passes, submits and downloads) does not depend on how fast a machine is.
It is a snapshot test instead, `oneiroi-bench/tests/counters.rs`, which runs
with the other tests. After an intended change, `cargo insta review`
accepts the new snapshot.
