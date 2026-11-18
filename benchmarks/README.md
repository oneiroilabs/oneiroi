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

## Where the time goes

Below the table of every scenario, each edit is split up the same way for
every backend:

- `set`: setting the new value, which hashes it,
- `plan`: finding out what has to run,
- `flush`: the executor, and of which
  - on the CPU: `kernels` (the nodes themselves) and `store` (writing what
    they made),
  - on the GPU: `upload`, `record` (writing the calls and encoding the
    passes), `submit`, `download`, and `cpu` for nodes which run on the
    CPU after all,
  - `rest`: what is left, on the GPU finding out sizes, making what the
    CPU makes, and making room in the store,
- `wait`: waiting for the device to finish,
- `device`: how long the GPU spent on compute passes, by its timestamps.

The executors only time parts which do not contain each other, so they
add up to `flush`.

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
