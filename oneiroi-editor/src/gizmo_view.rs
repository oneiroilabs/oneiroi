//! Gizmos drawn over the preview: the handles and guides of the selected
//! node, seen through the camera of the preview, and dragged in space.
//!
//! The overlay is a canvas of its own on top of the preview, so the
//! renderer of the preview knows nothing of it. It only takes the left
//! button where it hits a handle; orbiting and zooming go through to the
//! preview.

use std::sync::{Arc, Mutex};

use glam::{Mat4, Vec2, Vec3};
use iced::{
    Color, Point, Rectangle, Renderer, Size, Theme, keyboard, mouse,
    widget::canvas::{self, Event, Frame, Geometry, Path, Stroke},
};
use oneiroi_nodes::meta::gizmo::{Constraint, Gizmo, Guide, Point as Point3};
use oneiroi_wgpu::orbit::OrbitCamera;

use crate::document::Session;

/// How close to a handle a click picks it, in pixels.
const PICK: f32 = 8.0;

/// The gizmo of the selected node, over the preview. It publishes which
/// handle is dragged where.
pub struct GizmoOverlay {
    pub gizmo: Gizmo,
    pub session: Arc<Mutex<Session>>,
}

/// A handle being dragged: where it was, and what it can move in.
#[derive(Debug, Clone, Copy)]
struct Drag {
    handle: usize,
    from: Vec3,
    /// Where the cursor was on what it moves in when the drag started, so
    /// the handle keeps its distance to the cursor.
    grab: Vec3,
    space: Space,
}

#[derive(Debug, Default)]
pub struct State {
    drag: Option<Drag>,
    /// Shift locks a free handle to the axis it is dragged most along.
    shift: bool,
}

/// What a dragged handle moves in.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Space {
    Plane { normal: Vec3 },
    Axis { direction: Vec3 },
}

/// How the camera sees the canvas it is drawn on.
#[derive(Debug, Clone, Copy)]
pub struct Lens {
    view_projection: Mat4,
    size: Vec2,
    eye: Vec3,
    forward: Vec3,
}

impl Lens {
    pub fn new(camera: &OrbitCamera, size: Size) -> Self {
        let size = Vec2::new(size.width.max(1.0), size.height.max(1.0));
        Self {
            view_projection: camera.view_projection(size.x / size.y),
            size,
            eye: camera.position(),
            forward: (camera.target() - camera.position()).normalize_or(Vec3::Z),
        }
    }

    /// Where `point` is seen on the canvas, unless it is behind the camera.
    pub fn project(&self, point: Vec3) -> Option<Vec2> {
        let clip = self.view_projection * point.extend(1.0);
        if clip.w <= 1e-6 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(Vec2::new((ndc.x * 0.5 + 0.5) * self.size.x, (0.5 - ndc.y * 0.5) * self.size.y))
    }

    /// The ray from the camera through `position` on the canvas, as where it
    /// starts and where it heads.
    pub fn ray(&self, position: Vec2) -> (Vec3, Vec3) {
        let ndc = Vec2::new(position.x / self.size.x * 2.0 - 1.0, 1.0 - position.y / self.size.y * 2.0);
        let inverse = self.view_projection.inverse();
        // The depth is reversed: 1 is the near plane, and less is further.
        let near = inverse.project_point3(ndc.extend(1.0));
        let far = inverse.project_point3(ndc.extend(0.5));
        (near, (far - near).normalize_or(self.forward))
    }

    /// Where the ray through `position` meets `space` through `at`.
    fn on(&self, space: Space, at: Vec3, position: Vec2) -> Option<Vec3> {
        let (origin, direction) = self.ray(position);
        match space {
            Space::Plane { normal } => {
                let facing = direction.dot(normal);
                if facing.abs() < 1e-6 {
                    return None;
                }
                let t = (at - origin).dot(normal) / facing;
                (t > 0.0).then(|| origin + direction * t)
            }
            // The point of the axis closest to the ray.
            Space::Axis { direction: axis } => {
                let across = origin - at;
                let (a, b) = (axis.dot(direction), axis.dot(across));
                let (c, d) = (direction.dot(across), 1.0 - a * a);
                if d.abs() < 1e-6 {
                    return None;
                }
                Some(at + axis * ((b - a * c) / d))
            }
        }
    }
}

/// What a handle with `constraint` moves in, seen along `forward`.
fn space(constraint: Constraint, forward: Vec3) -> Space {
    match constraint {
        Constraint::Free => Space::Plane { normal: forward },
        Constraint::Plane(normal) => Space::Plane { normal: Vec3::from_array(normal).normalize_or(Vec3::Z) },
        Constraint::Axis(direction) => Space::Axis { direction: Vec3::from_array(direction).normalize_or(Vec3::X) },
    }
}

/// The axis of the world `moved` goes along the most.
fn main_axis(moved: Vec3) -> Vec3 {
    let abs = moved.abs();
    match (abs.x >= abs.y && abs.x >= abs.z, abs.y >= abs.z) {
        (true, _) => Vec3::X,
        (false, true) => Vec3::Y,
        (false, false) => Vec3::Z,
    }
}

impl GizmoOverlay {
    fn lens(&self, bounds: Rectangle) -> Option<Lens> {
        let camera = self.session.lock().unwrap().camera?;
        Some(Lens::new(&camera, bounds.size()))
    }

    /// The handle under `position`, the closest if there are several.
    fn picked(&self, lens: &Lens, position: Vec2) -> Option<usize> {
        self.gizmo
            .handles
            .iter()
            .enumerate()
            .filter_map(|(index, handle)| Some((index, lens.project(Vec3::from_array(handle.at))?.distance(position))))
            .filter(|(_, distance)| *distance <= PICK)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }
}

impl canvas::Program<(usize, Point3)> for GizmoOverlay {
    type State = State;

    fn update(
        &self,
        state: &mut State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<(usize, Point3)>> {
        if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
            state.shift = modifiers.shift();
            return None;
        }
        let lens = self.lens(bounds)?;
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds)?;
                let position = Vec2::new(position.x, position.y);
                let handle = self.picked(&lens, position)?;
                let from = Vec3::from_array(self.gizmo.handles[handle].at);
                let space = space(self.gizmo.handles[handle].constraint, lens.forward);
                let grab = lens.on(space, from, position).unwrap_or(from);
                state.drag = Some(Drag { handle, from, grab, space });
                Some(canvas::Action::request_redraw().and_capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let drag = state.drag?;
                // Outside the canvas too, so a handle can be dragged as far
                // as the window goes.
                let position = cursor.position()?;
                let position = Vec2::new(position.x - bounds.x, position.y - bounds.y);
                let mut to = drag.from + lens.on(drag.space, drag.from, position)? - drag.grab;
                let free = self.gizmo.handles[drag.handle].constraint == Constraint::Free;
                if state.shift && free {
                    let axis = main_axis(to - drag.from);
                    to = drag.from + axis * (to - drag.from).dot(axis);
                }
                Some(canvas::Action::publish((drag.handle, to.to_array())).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.drag.take()?;
                Some(canvas::Action::request_redraw().and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let Some(lens) = self.lens(bounds) else {
            return vec![frame.into_geometry()];
        };
        let palette = theme.palette();
        let guide = Stroke::default().with_width(1.5).with_color(Color {
            a: 0.8,
            ..palette.primary.weak.color
        });
        let point = |at: Vec2| Point::new(at.x, at.y);
        let polyline = |frame: &mut Frame, points: &[Vec3]| {
            let seen: Vec<Option<Vec2>> = points.iter().map(|point| lens.project(*point)).collect();
            for pair in seen.windows(2) {
                if let [Some(a), Some(b)] = pair {
                    frame.stroke(&Path::line(point(*a), point(*b)), guide);
                }
            }
        };
        for shown in &self.gizmo.guides {
            match shown {
                Guide::Polyline(points) => {
                    let points: Vec<Vec3> = points.iter().map(|point| Vec3::from_array(*point)).collect();
                    polyline(&mut frame, &points);
                }
                Guide::Circle { center, radius, normal } => {
                    let (center, normal) = (Vec3::from_array(*center), Vec3::from_array(*normal).normalize_or(Vec3::Z));
                    let (u, v) = normal.any_orthonormal_pair();
                    let points: Vec<Vec3> = (0..=64)
                        .map(|step| {
                            let angle = step as f32 / 64.0 * std::f32::consts::TAU;
                            center + (u * angle.cos() + v * angle.sin()) * *radius
                        })
                        .collect();
                    polyline(&mut frame, &points);
                }
            }
        }

        let hovered = cursor
            .position_in(bounds)
            .and_then(|position| self.picked(&lens, Vec2::new(position.x, position.y)));
        let dragged = state.drag.map(|drag| drag.handle);
        for (index, handle) in self.gizmo.handles.iter().enumerate() {
            let Some(at) = lens.project(Vec3::from_array(handle.at)) else {
                continue;
            };
            let active = hovered == Some(index) || dragged == Some(index);
            let size = if active { 10.0 } else { 8.0 };
            let corner = Point::new(at.x - size / 2.0, at.y - size / 2.0);
            let fill = match active {
                true => palette.primary.strong.color,
                false => palette.background.base.text,
            };
            frame.fill_rectangle(corner, Size::new(size, size), Color::BLACK);
            let inner = Point::new(corner.x + 1.0, corner.y + 1.0);
            frame.fill_rectangle(inner, Size::new(size - 2.0, size - 2.0), fill);
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, state: &State, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if state.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        let over = self.lens(bounds).zip(cursor.position_in(bounds)).and_then(|(lens, position)| {
            self.picked(&lens, Vec2::new(position.x, position.y))
        });
        match over {
            Some(_) => mouse::Interaction::Grab,
            None => mouse::Interaction::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lens() -> Lens {
        let mut camera = OrbitCamera::default();
        camera.yaw_pitch_delta(0.4, -0.3);
        Lens::new(&camera, Size::new(800.0, 600.0))
    }

    #[test]
    fn a_ray_goes_through_what_is_seen_there() {
        let lens = lens();
        for point in [Vec3::ZERO, Vec3::new(0.5, -0.3, 0.2), Vec3::new(-1.0, 1.0, 0.5)] {
            let seen = lens.project(point).unwrap();
            let (origin, direction) = lens.ray(seen);
            // The point is on the ray: as far from it as rounding goes.
            let along = (point - origin).dot(direction);
            assert!((origin + direction * along).distance(point) < 1e-3, "{point}");
        }
    }

    #[test]
    fn a_handle_is_dragged_to_where_the_cursor_is_in_its_space() {
        let lens = lens();
        let at = Vec3::new(0.2, 0.1, 0.0);
        let target = Vec3::new(0.7, -0.4, 0.0);
        // In the plane of `x` and `y`, it goes right where the cursor points.
        let plane = space(Constraint::Plane([0.0, 0.0, 1.0]), lens.forward);
        let to = lens.on(plane, at, lens.project(target).unwrap()).unwrap();
        assert!(to.distance(target) < 1e-3, "{to}");
        // Along `x`, only its `x` changes.
        let axis = space(Constraint::Axis([1.0, 0.0, 0.0]), lens.forward);
        let to = lens.on(axis, at, lens.project(target).unwrap()).unwrap();
        assert!((to.y - at.y).abs() < 1e-5 && (to.z - at.z).abs() < 1e-5);
        assert!(to.x > at.x);
        // Free, it stays as far from the camera as it was, seen along it.
        let free = space(Constraint::Free, lens.forward);
        let to = lens.on(free, at, lens.project(target).unwrap()).unwrap();
        assert!(((to - at).dot(lens.forward)).abs() < 1e-4);
    }

    #[test]
    fn shift_takes_the_axis_moved_along_the_most() {
        assert_eq!(main_axis(Vec3::new(0.1, -0.5, 0.2)), Vec3::Y);
        assert_eq!(main_axis(Vec3::new(2.0, 1.0, -1.0)), Vec3::X);
    }
}
