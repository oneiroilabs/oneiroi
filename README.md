# Oneiroi (WIP)
An engine-agnostic, node-based, non-destructive procedural modeling library written in Rust.

## What does that mean?
After that buzzword bingo it is no bad idea to untangle that sentence and define the parts that make it up:
- engine-agnostic: Oneiroi is optimized for real-time applications but just exposes the minimal API to allow easy integration into engines.
- node-based: Users connect nodes that execute operations on data-types to build their 3D models.
- non-destrictive: Every node can always be edited, changed or reconnected to alter the produced models.
- modeling-library: As mentioned before this library enables you to build 3D-models. They can also be altered at runtime through exposed variables.  

