use makepad_vector::geometry::Point;

/// Converts a type to another type.
pub(crate) trait Convert<T> {
    #[must_use]
    fn convert(self) -> T;
}

impl<const N: usize, O, I: Convert<O>> Convert<[O; N]> for [I; N] {
    fn convert(self) -> [O; N] {
        self.map(I::convert)
    }
}

impl Convert<Point> for lyon_path::math::Point {
    fn convert(self) -> Point {
        Point::new(self.x as f64, self.y as f64)
    }
}
