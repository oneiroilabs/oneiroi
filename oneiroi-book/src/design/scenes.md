# Scenes

A scene has many graphs, and many instances of each. This is how they run
together, what that costs, and how one instance follows another.

## One frame

`Engine::evaluate(instances)` plans every instance it is given, and then
runs all of it as one batch:
- **Planning:**
  - Each instance adds only what the store is missing.
  - Work which is the same for two instances, or two graphs, has the same
    key and is planned once.
- **Settled instances:** an instance whose outputs are all there holds them,
  the way it holds its parameters. Until a parameter changes, planning skips
  it altogether.
- **Levels:** the batch is sorted into levels by what reads what.
- **One dispatch per node:** on the GPU, all calls of one node in a level
  are one dispatch, whichever graph or instance they come from.

## What it costs

| Grows with | Does not grow with |
|---|---|
| Instances which changed, and the elements they make | Instances which did not change |
| Kinds of nodes × levels: the dispatches | Graphs which use the same kinds of nodes |
| Calls in the batch: their keys, their room in the store, their part of the table of calls | Instances or parts of graphs which are the same |

On the CPU, a call costs:
- a key for each output;
- a look into the store for each input;
- room for each output.

The executor keeps the calls of a node flat, and makes room for all outputs
of a dispatch in one go. Nothing is allocated per call.

**Eviction:** a value which is held is taken out of the line of eviction the
first time eviction finds it. Ending a frame then only checks whether its
holders still live.

### Measured

The benchmark scenarios `scene, a few move` and `scene, all move` have
instances of different graphs. Each graph makes a curve out of the points of
its instance and moves it, then sweeps it, samples it or moves it again with
constants of its own. A size is the instances × the graphs.

These numbers are from an edit at 16384 × 64, with a software GPU (lavapipe)
and an Intel Xeon at 2.1 GHz.

**A few move:** 4 instances move.

| | before | after |
|---|---:|---:|
| plan | 1.70 ms | 0.14 ms |
| end of the frame | 1.27 ms | 0.23 ms |

**All move:** every instance moves.

| | before | after |
|---|---:|---:|
| flush, GPU | 55 ms | 28 ms |
| plan | 15.6 ms | 15.9 ms |

Planning a changed instance stays at about 1 µs.

From 4096 × 16 to 4096 × 64 (four times the graphs) the GPU does the same
passes. Its time grows only by the calls of the graphs which differ.

## Following another instance

A parameter can follow an output of another instance, like a fence which
follows the curve of a road:

```rust
engine.link(&mut fence, "path", &road, "curve")?;
```

**No copy:**
- The parameter gets the key of the output.
- Whatever the road makes is read by the fence where it is, on the GPU too.

**Order:**
- In one `plan`, whatever follows comes after what it follows.
- An edit of the road reaches the fence in the same frame.

**Without its source:**
- Planned on its own, the fence follows what the road was planned with
  last.
- That is made again if it was evicted, even once the road is gone:
  whatever is followed keeps its parameters held for as long as somebody
  follows it.

**What comes along:**
- The groups of the parameter follow those of the output.
- An output without groups is a single group.
- The shape instances refer to, and their total, follow too, or the link is
  refused.

**Refused links:**
- a parameter of another type than the output;
- a link that would go round in a circle.

**Ending it:** setting the parameter, or `Engine::unlink`, ends a link. The
parameter keeps the value it has.

**How it is found:**
- Each instance which is followed has a small cell, made the first time
  something follows it. It holds what the instance was last planned with,
  and a version.
- A follower only looks inside when the version changed.
- Instances nobody follows never touch a cell.

The scenario `scene, linked` measures this. It has rails, each followed by a
tube which is swept along it, and 4 rails move. At 16384 rails and 16384
tubes, planning the 32768 instances takes 0.95 ms, about 30 ns each, mostly
spent reading the cells.

## Next

- **Cheaper calls:** what is left per call is mostly looking up inputs in
  the store. Handing the executor the handles the planner already found
  would save most of it.
- **The allocator:** room is found in a tree of free runs, at four
  operations for each value. Lists of free runs by size, for small values,
  would make that one.
- **Planning many changes:** an edit of every instance costs about 1 µs
  each.
