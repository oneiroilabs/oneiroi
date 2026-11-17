//! Holds a budget of triangles by changing how fine things are drawn.

/// Finds the step size which makes about `triangles` triangles, and
/// follows it as what is seen changes.
///
/// Every update moves the step size a bit towards where it should be, but
/// never by much at once, so the detail changes gradually rather than
/// jumping. Close to the budget it is left alone, which keeps it from
/// going back and forth.
#[derive(Debug, Clone, PartialEq)]
pub struct DetailBudget {
    /// How many triangles there should be.
    pub triangles: u32,
    /// How long a step is on screen right now, in pixels.
    pixels: f32,
    /// The finest and the coarsest it may get.
    pub range: (f32, f32),
}

impl DetailBudget {
    /// How much the step size may change from one update to the next.
    const STEP: f32 = 1.15;
    /// How far from the budget is close enough.
    const SLACK: f32 = 0.1;

    pub fn new(triangles: u32, pixels: f32) -> Self {
        Self {
            triangles,
            pixels,
            range: (1.0, 256.0),
        }
    }

    pub fn pixels(&self) -> f32 {
        self.pixels
    }

    /// Takes in how many triangles there were with the current step size
    /// and returns the next one.
    pub fn update(&mut self, triangles: u32) -> f32 {
        let ratio = triangles.max(1) as f32 / self.triangles.max(1) as f32;
        if (ratio - 1.0).abs() > Self::SLACK {
            // Triangles grow with the square of the steps per side.
            let change = ratio.sqrt().clamp(1.0 / Self::STEP, Self::STEP);
            self.pixels = (self.pixels * change).clamp(self.range.0, self.range.1);
        }
        self.pixels
    }

    /// Whether the last update left the step size where it was.
    pub fn settled(&self, triangles: u32) -> bool {
        let ratio = triangles.max(1) as f32 / self.triangles.max(1) as f32;
        let at_limit = (ratio > 1.0 && self.pixels >= self.range.1)
            || (ratio < 1.0 && self.pixels <= self.range.0);
        (ratio - 1.0).abs() <= Self::SLACK || at_limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scene in which the triangles grow with the square of how many
    /// steps there are per pixel.
    fn scene(size: f32) -> impl Fn(f32) -> u32 {
        move |pixels| (size / (pixels * pixels)) as u32
    }

    #[test]
    fn the_budget_is_found_gradually() {
        let triangles = scene(1_000_000.0);
        let mut budget = DetailBudget::new(10_000, 2.0);
        let mut pixels = budget.pixels();
        for _ in 0..60 {
            let next = budget.update(triangles(pixels));
            assert!(next / pixels <= 1.15 + 1e-4 && pixels / next <= 1.15 + 1e-4);
            pixels = next;
        }
        let made = triangles(pixels);
        assert!(budget.settled(made));
        assert!((made as f32 / 10_000.0 - 1.0).abs() <= 0.1, "{made}");
    }

    #[test]
    fn it_follows_what_is_seen_and_then_stays() {
        let mut budget = DetailBudget::new(10_000, 10.0);
        let mut pixels = budget.pixels();
        for _ in 0..60 {
            pixels = budget.update(scene(1_000_000.0)(pixels));
        }
        // Something larger comes into view: coarser steps.
        let before = pixels;
        for _ in 0..60 {
            pixels = budget.update(scene(4_000_000.0)(pixels));
        }
        assert!((pixels / before - 2.0).abs() < 0.25, "{before} {pixels}");

        // Once there, it does not move.
        let settled = pixels;
        for _ in 0..10 {
            pixels = budget.update(scene(4_000_000.0)(pixels));
        }
        assert_eq!(pixels, settled);
    }

    #[test]
    fn it_stays_within_its_range() {
        let mut budget = DetailBudget::new(10, 4.0);
        budget.range = (2.0, 8.0);
        for _ in 0..50 {
            budget.update(1_000_000);
        }
        assert_eq!(budget.pixels(), 8.0);
        assert!(budget.settled(1_000_000));
        for _ in 0..50 {
            budget.update(0);
        }
        assert_eq!(budget.pixels(), 2.0);
    }
}
