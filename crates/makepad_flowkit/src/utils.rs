use makepad_vector::geometry::Point;

/// Converts a type to another type.
pub trait Convert<T> {
    #[must_use]
    fn convert(self) -> T;
}

impl Convert<Point> for lyon_path::math::Point {
    fn convert(self) -> Point {
        Point::new(self.x as f64, self.y as f64)
    }
}
