//! A small editor for a curve drawn as a graph over `x`: its points can be
//! dragged, a click where there is none adds one, and a right click on one
//! takes it out. What it edits is a [`CurveDrawing`].

use glam::Vec2;
use iced::{
    Color, Point, Rectangle, Renderer, Size, Theme, mouse,
    widget::canvas::{self, Event, Frame, Geometry, Path, Stroke, Text},
};

use crate::document::CurveDrawing;

/// How far in from the edges the graph is, in pixels.
const MARGIN: f32 = 10.0;
/// How close to a point a click picks it, in pixels.
const PICK: f32 = 7.0;
/// A curve keeps at least this many points.
const FEWEST: usize = 2;

/// The editor of `drawing`. It publishes the points whenever they change.
pub struct CurveEdit {
    pub drawing: CurveDrawing,
}

/// Which point is being dragged.
#[derive(Debug, Default)]
pub struct State {
    dragged: Option<usize>,
}

/// Where the graph is drawn: the part of the plane it shows, and the
/// pixels it shows it in.
struct View {
    /// The lowest and highest `x` and `y` it shows.
    min: Vec2,
    max: Vec2,
    bounds: Rectangle,
}

impl View {
    /// `x` from `0` to `1` and `y` from `0` to `1`, and more where a point
    /// lies beyond them.
    fn of(points: &[Vec2], bounds: Rectangle) -> Self {
        let (min, max) = points
            .iter()
            .fold((Vec2::ZERO, Vec2::ONE), |(min, max), point| (min.min(*point), max.max(*point)));
        Self { min, max, bounds }
    }

    fn inner(&self) -> Rectangle {
        Rectangle {
            x: MARGIN,
            y: MARGIN,
            width: (self.bounds.width - 2.0 * MARGIN).max(1.0),
            height: (self.bounds.height - 2.0 * MARGIN).max(1.0),
        }
    }

    /// Where `point` is drawn, in the pixels of the canvas.
    fn to_screen(&self, point: Vec2) -> Point {
        let inner = self.inner();
        let at = (point - self.min) / (self.max - self.min).max(Vec2::splat(1e-6));
        Point::new(inner.x + at.x * inner.width, inner.y + (1.0 - at.y) * inner.height)
    }

    /// The point drawn at `position`, in the pixels of the canvas.
    fn to_plane(&self, position: Point) -> Vec2 {
        let inner = self.inner();
        let at = Vec2::new((position.x - inner.x) / inner.width, 1.0 - (position.y - inner.y) / inner.height);
        self.min + at * (self.max - self.min)
    }
}

impl CurveEdit {
    /// The point under `position`, if there is one close enough.
    fn picked(&self, view: &View, position: Point) -> Option<usize> {
        self.drawing
            .points
            .iter()
            .enumerate()
            .map(|(index, point)| (index, view.to_screen(*point).distance(position)))
            .filter(|(_, distance)| *distance <= PICK)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }
}

/// A point dragged or added to `x` stays in `0..=1`, where the curve is
/// read: further along a sweep than its end is nothing.
fn kept_in(point: Vec2) -> Vec2 {
    Vec2::new(point.x.clamp(0.0, 1.0), point.y)
}

impl canvas::Program<Vec<Vec2>> for CurveEdit {
    type State = State;

    fn update(
        &self,
        state: &mut State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Vec<Vec2>>> {
        let view = View::of(&self.drawing.points, bounds);
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds)?;
                if let Some(index) = self.picked(&view, position) {
                    state.dragged = Some(index);
                    return Some(canvas::Action::request_redraw().and_capture());
                }
                // A new point, among the others by its `x`.
                let point = kept_in(view.to_plane(position));
                let mut points = self.drawing.points.clone();
                let index = points.partition_point(|other| other.x <= point.x);
                points.insert(index, point);
                state.dragged = Some(index);
                Some(canvas::Action::publish(points).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let position = cursor.position_in(bounds)?;
                let index = self.picked(&view, position)?;
                if self.drawing.points.len() <= FEWEST {
                    return Some(canvas::Action::capture());
                }
                let mut points = self.drawing.points.clone();
                points.remove(index);
                Some(canvas::Action::publish(points).and_capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let index = state.dragged?;
                // Also outside of the canvas, so a point can be dragged
                // beyond what it shows, which then shows more.
                let position = cursor.position()?;
                let position = Point::new(position.x - bounds.x, position.y - bounds.y);
                let mut points = self.drawing.points.clone();
                *points.get_mut(index)? = kept_in(view.to_plane(position));
                Some(canvas::Action::publish(points).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragged.take()?;
                Some(canvas::Action::request_redraw())
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
        let palette = theme.palette();
        let view = View::of(&self.drawing.points, bounds);
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), palette.background.weak.color);

        // A line every quarter along `x`, and at `0` and `1` up `y`.
        let faint = Color {
            a: 0.25,
            ..palette.background.base.text
        };
        let grid = Stroke::default().with_width(1.0).with_color(faint);
        let line = |from: Vec2, to: Vec2| Path::line(view.to_screen(from), view.to_screen(to));
        for quarter in 0..=4 {
            let x = quarter as f32 / 4.0;
            frame.stroke(&line(Vec2::new(x, view.min.y), Vec2::new(x, view.max.y)), grid);
        }
        // Each labelled inside the graph: 1 under its line, 0 over it.
        for (y, below) in [(0.0, -12.0), (1.0, 1.0)] {
            frame.stroke(&line(Vec2::new(view.min.x, y), Vec2::new(view.max.x, y)), grid);
            frame.fill_text(Text {
                content: format!("{y}"),
                position: view.to_screen(Vec2::new(view.min.x, y)) + iced::Vector::new(3.0, below),
                color: faint,
                size: 10.0.into(),
                ..Text::default()
            });
        }

        let curve = self.drawing.line();
        if curve.len() > 1 {
            let path = Path::new(|builder| {
                builder.move_to(view.to_screen(curve[0]));
                for point in &curve[1..] {
                    builder.line_to(view.to_screen(*point));
                }
            });
            frame.stroke(&path, Stroke::default().with_width(2.0).with_color(palette.primary.base.color));
        }

        let hovered = cursor.position_in(bounds).and_then(|position| self.picked(&view, position));
        for (index, point) in self.drawing.points.iter().enumerate() {
            let active = state.dragged == Some(index) || hovered == Some(index);
            let size = if active { 9.0 } else { 7.0 };
            let center = view.to_screen(*point);
            let color = match active {
                true => palette.primary.strong.color,
                false => palette.background.base.text,
            };
            frame.fill_rectangle(center - iced::Vector::new(size / 2.0, size / 2.0), Size::new(size, size), color);
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, state: &State, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        let view = View::of(&self.drawing.points, bounds);
        match (state.dragged, cursor.position_in(bounds)) {
            (Some(_), _) => mouse::Interaction::Grabbing,
            (None, Some(position)) if self.picked(&view, position).is_some() => mouse::Interaction::Grab,
            (None, Some(_)) => mouse::Interaction::Crosshair,
            (None, None) => mouse::Interaction::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_is_drawn_where_it_is_picked() {
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(220.0, 120.0));
        let view = View::of(&[Vec2::new(0.0, -1.0), Vec2::new(1.0, 2.0)], bounds);
        // Beyond `0..=1` it shows more.
        assert_eq!((view.min, view.max), (Vec2::new(0.0, -1.0), Vec2::new(1.0, 2.0)));
        for point in [Vec2::ZERO, Vec2::new(0.25, 1.5), Vec2::new(1.0, -1.0)] {
            assert!(view.to_plane(view.to_screen(point)).distance(point) < 1e-5);
        }
        // Up is up.
        assert!(view.to_screen(Vec2::Y).y < view.to_screen(Vec2::ZERO).y);
    }
}
