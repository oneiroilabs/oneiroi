# Constants

In the editor every value of a graph can change. Outside of it, only the
values a graph exposes as parameters can: the rest become constants of its
program when it is lowered. Usually most of a graph is constant, and nothing
of that is paid for again on an edit.

## The constant part of a program

The first time a program is planned, the engine finds what of it depends on
no parameter. This is `Statics`, kept once per program:
- **the keys** of every slot which depends on no parameter, including whole
  calls with only constant arguments;
- **for each parameter**, the operations which depend on it;
- **the edge**: the constant slots read by operations which depend on a
  parameter, and the constant outputs with what comes along with them, like
  groups.

**Keys.**
- A new instance starts from the constant keys and derives only the rest.
- Setting a parameter marks it as changed. The next plan derives again only
  the operations which depend on it, in their order.
- The keys always equal what deriving all of them gives; a test checks it edit
  after edit.

**Pinning the edge.**
- Once the program ran for the first time, its edge is pinned in the store
  (`Store::pin`) for as long as the program is there.
- Pinned values are out of the line of eviction altogether: evicting only
  checks, once per program, whether the program is still there.
- Everything behind the edge, which only the constant part read, may be evicted
  freely. It is never needed again.
- Planning then skips the constant part without even looking it up: only the
  outputs which depend on a parameter are planned.
- When the program is removed, its token dies and what it pinned goes back in
  line, as the oldest there is.

**What it changed.** The benchmark scenario *mostly constant* has as many
constant curves as its size, each an output, and one curve moved by the only
parameter. On one CPU thread, an edit took:

| constant curves | before | after |
|---|---:|---:|
| 256 | 20 µs | 0.9 µs |
| 4096 | 702 µs | 1.0 µs |
| 16384 | 2.47 ms | 2.5 µs |

Ending the frame went from 332 µs to 0.3 µs at 16384 curves. Before, planning
and eviction looked at all of the constant part on every edit.

## Folding at load

A host which loads a graph and never edits it can go one step further:
`Engine::fold(program)`.
- It runs the constant part once, on whatever executor the engine has, and
  downloads what is needed from the device.
- It makes a new program with only the operations which depend on a parameter,
  and the values they read in place of the constant part, as constants.
- The folded program has the same parameters and outputs. Its constants keep
  the keys they were derived under, so it gives the same results under the
  same keys as the program it was folded from, and shares them with it.

It is a step of its own rather than the default: it needs an engine at load
time, and an editor which changes the graph again would only fold it again.
