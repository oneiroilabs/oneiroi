# One item per node

A node that makes a shape makes one: one circle, one rectangle, one curve,
one surface. Many shapes come from two places that say so: **Instance on
Points** puts copies of one shape onto points, and running a graph **for
each** element of a list makes a different shape per element. The batching
the GPU needs stays in the engine: it is no longer something an artist has
to think about.

## Why

Every value is stored as a flat array, so it was easy to write every kernel
over a list. Most shape nodes therefore took lists: Circle made a circle per
radius, Line a line per pair of points, and NURBS a curve per group of control
points. This had two costs:

- **Nobody asked for it.** A graph almost always wants one circle. The batches
  were there for the engine and the benchmarks.
- **A list of shapes was written two ways.** A circle fits in one element, so
  a list of circles was a plain list. A curve is a group of segments, so a list
  of curves was a list of groups. The sockets showed `[●]` for one and `{●}`
  for the other, though both meant "some shapes".

## Elements, parts and items

- An **element** is one entry of the array a value is stored in: a number, a
  vector, a circle, a frame, a segment, a patch.
- Some types are **made of parts** (`TypeInfo::parts`): an element is only a
  part of a shape. A curve is all of its segments, and a surface all of its
  patches.
- An **item** is what an artist counts:
  - for most types, an element;
  - for a type made of parts, a whole shape.

A port has a `Card`, which says how its kernel sees the elements:

| `Card`    | The kernel gets |
|-----------|-----------------|
| `One`     | a single element |
| `Many`    | a list of elements |
| `Grouped` | elements, plus the offsets where each group starts |

What that means to whoever links it is `Card::items`:

| `Card`    | Most types            | Types made of parts  |
|-----------|-----------------------|----------------------|
| `One`     | one item              | one shape            |
| `Many`    | a list                | one shape            |
| `Grouped` | a list of lists       | a list of shapes     |

Links are checked by items:
- One item goes anywhere.
- Only one item goes where one is wanted.
- A list of lists goes into a list as all of its elements.

The editor draws a socket by its items:
- `●` for one;
- `[●]` for a list, of numbers or of shapes alike;
- `{●}` only for a real list of lists.

## The rules

1. **A node that makes a shape makes one.**

   Its inputs that describe the shape take one item: a radius, a start and an
   end, a curve. Inputs that are naturally lists stay lists, like the control
   points of one curve or the knots.

2. **A value node works on lists, and gives one item for one item.**

   - Numbers, Vectors, Math and Combine XYZ take lists and give a list,
     element by element, so a few heights for the rails are typed once.
   - Their outputs *follow* their inputs (`PortDecl::follows`): when every
     input they follow is one item, the output is one item. So Math of two
     single numbers can go into the end of a Line.

3. **Modifiers and conversions keep the count.**

   Translate Curve, To Curve and the conversion of points into frames make an
   item per item: one curve in gives one curve out, a list gives a list.

4. **Many shapes are asked for.**

   - Instance on Points copies one shape (or a list of them) onto each point.
     Points are frames, and plain vectors become frames on their own (the new
     conversion **Points to Frames**), so a shape can go onto any vectors.
   - A graph used as a node can run for each element of an input. That makes
     a different shape per element.

## The nodes

| Node | Inputs | Output |
|---|---|---|
| Circle | `radius`: one | `circle`: one |
| Rectangle | `width`, `height`, `corner_radius`: one each | `rectangle`: one |
| Line | `start`, `end`: one each | `curve`: one |
| Polyline | `points`: a list; `closed`: one | `curve`: one |
| NURBS Curve | `points`, `knots`: a list each | `segments`: one curve |
| Sample Curve | `segments`: one curve; `count`, `spacing`: one each; `t`: a list | `frames`: a list |
| Sweep | `spine`, `profile`, `scale`: one each; `ends`: one | `surface`: one |
| Translate Curve | `segments`: a curve or a list of them; `offset`: one | as many as came in |
| To Curve | `shape`: a shape or a list | as many curves |
| Points to Frames | `points`: a list | a frame per point |
| Mesh | `surface`: one or a list | one mesh |
| Instance on Points | `points`: a list; `shape`: one or a list | the copies |
| Numbers, Vectors, Math, Combine XYZ | lists | a list, one item for one item |

## The batches

The engine still runs one kernel over many shapes where it knows there are many.
Each single node with a GPU kernel has a **batched form**, hidden from the
palette:
- `curve.nurbs.batch`
- `curve.sample.batch`
- `curve.sweep.batch`, a version for each profile

Both forms share their CPU and GPU kernels:
- **CPU:** an input without offsets reads as a single group of everything.
- **GPU:** the shader generator now writes the group functions
  (`in_x_groups`, `in_x_start`, `in_x_end`, `in_x_group_of`) for every input,
  and an input without offsets is that same single group.
- **Outputs:** an output without offsets ignores `Outputs::set_groups`.

So the single form is the batched one run on one shape, and its results are the
same.

The benchmarks run the batches, since they measure hundreds of curves in one
call. A graph uses the single forms.

## The fence

The fence shows the rules:
- **Rails:** one rail is a line as long as the fence, swept once and placed at
  three heights.
- **Posts and caps:** one post and one pointed cap, each placed at both ends.
- **Pickets:** made once and placed along the fence, as before.

Each rail and post is now swept once and only placed, so an edit of the length
runs fewer nodes than before.

## What changed for graphs

- A list linked to an input that takes one item is an error now. Before, such
  an input took the list.
- Saved graphs which typed a list into such an input need one value per node
  instead. To get several shapes, link a list of points into Instance on Points.
- The single nodes counted their version up, so no result of the old ones is
  mistaken for theirs.

## Next

1. **Batch for-each.** Running a graph for each element still runs it once per
   element. When its body is single nodes which have batched forms, it can be
   lowered to one call of the batches over all elements instead. The each-curve
   benchmark measures this.
2. **Show flattening.** Draw a list of lists going into a list differently on
   the link, and offer an explicit Flatten node.

## Curves drawn at an input

The scale of a sweep is a curve whose `x` is how far along the spine and whose
`y` is how big the profile is there.

**Curve 2D** draws such a curve:
- **Points:** you drag the points in a small editor, click to add one and right
  click to take one out. They are taken from left to right.
- **Interpolation:** how it gets from point to point.
  - `smooth` goes through every point without overshooting between two of
    them (the slopes of Fritsch and Carlson).
  - `linear` goes straight.
  - `nurbs` is pulled by the points as the control points of a NURBS curve.
- **Result:** one NURBS curve like any other, so it fits wherever a curve goes.

A curve can also be drawn **right at an input** which takes one, without a
node in the graph:
- **Graph:** such an input holds an *inline node* (`Input::Inline`), a node
  only it sees, with values for the inputs of that node.
- **Lowering:** the inline node is built like any other node, and its first
  output goes into the input. A curve drawn inline and the same Curve 2D node
  linked give the same result, under the same key.
- **Editor:** the inspector shows the curve editor wherever a curve is drawn,
  and a "draw a curve" button on a free input which takes one.
- **Fence:** the pointed scales of the spears and the caps are drawn this way.
