use egui::Pos2;
use lyon_path::math::Point;

pub(crate) trait Convert {
    type Output;

    fn convert(self) -> Self::Output;
}

impl<T: Convert, const N: usize> Convert for [T; N] {
    type Output = [T::Output; N];

    fn convert(self) -> Self::Output {
        self.map(T::convert)
    }
}

impl Convert for Point {
    type Output = Pos2;

    fn convert(self) -> Self::Output {
        Pos2::new(self.x, self.y)
    }
}
