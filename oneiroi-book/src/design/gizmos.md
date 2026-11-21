# Gizmos

A gizmo is what the editor draws of a selected node in the viewport, to drag
instead of typing numbers:
- the control points of a NURBS curve, joined as its control polygon;
- the ends of a line;
- the radius of a circle;
- the corner of a rectangle;
- the offset of Translate Curve;
- typed-in vectors.

A drag changes the node's inputs, the same way typing into the inspector does,
and the preview follows.

## Where it lives

Only an editor needs gizmos, so none of them is in `oneiroi-core`. The engine
and every host which only runs graphs, like Godot, Bevy or C++, compile
nothing of them.

| Crate | What it has |
|---|---|
| `oneiroi-meta` | The plain data of a gizmo, `meta::gizmo`: `Handle`, `Guide`, `Constraint`, `Edit`, `GizmoInputs`, and `GizmoMeta`, which is two function pointers per node. It depends on nothing, like the rest of meta. |
| `oneiroi-nodes`, feature `meta` | The `Handled` trait over `DataType`, and the gizmo of each node, registered into the `MetaRegistry` next to its documentation. |
| `oneiroi-editor` | A canvas over the preview which draws gizmos with the preview's camera and turns drags into inputs. |

The functions work on the words of values, so meta needs no engine types.
Behind them, each node uses typed values: it reads its inputs as `Vec3`,
`ControlPoint` and so on, and writes them back as words.

## Declaring a gizmo

A value somewhere in space is `Handled`: it has a position, and a drag moves it
and keeps everything else:
- `Vec3` is the point it is.
- A `ControlPoint` moves its position and keeps its weight.
- A `Vec2` stays in the plane of `x` and `y`.

A node's gizmo is a `GizmoMeta`:
- **`show`** turns the inputs into handles and guides.
  - **Handles** are what can be dragged: each stands for an element of an
    input, and has a constraint.
  - **Guides** are drawn but not dragged, like a control polygon or the circle
    of a radius.
- **`drag`** turns "this handle, dragged to this point" into edits of the
  inputs.

Most gizmos are the points of one input, which `points::<T: Handled>` and
`moved::<T>` cover. The rest are a few lines each.

Constraints:
- **`Free`** moves in the plane through the handle that faces the camera. With
  Shift held, it moves along the world axis it is dragged along most.
- **`Axis(direction)`** moves along that line, like the radius of a circle
  along `x`.
- **`Plane(normal)`** moves in that plane, like the corner of a rectangle or a
  point of a Curve 2D.

## In the editor

- **What a gizmo sees:** the document gives it only the inputs of the selected
  node that are typed in or at their default. A linked input depends on
  another node, so the gizmo leaves it out.
- **A drag:**
  - The edits become values of the inputs through the same setters as the
    inspector.
  - Dragging a handle to where it already is changes nothing.
- **The overlay:**
  - It is an iced canvas stacked on the preview. The preview hands it the
    camera it drew with, through the session.
  - It projects handles and guides on the CPU, and casts a ray from the camera
    through the cursor to find where a dragged handle goes.
  - It takes only the left button on a handle, so orbiting and zooming still go
    to the preview.
  - Handles are drawn on top of everything, which is what editing wants.

## Limits, and what is next

- **Space:** handles are drawn in the node's own space. Where its result is put
  elsewhere, as in the pickets of the fence, they are drawn where they would
  be before that.
  - **Next:** follow the transforms of instances from the selected node to what
    is shown.
- **Linked inputs** are not shown.
  - **Next:**
    - show what they evaluate to, read-only;
    - drag the value node they come from;
    - draw outputs too, like the frames of Sample Curve as small axes.
