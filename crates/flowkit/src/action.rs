use glam::Vec2;

#[derive(Clone, Copy)]
pub enum Action {
    MoveTo(Vec2),
    LineTo(Vec2),
    ArcTo(Vec2, Vec2, f32),
    CubicBezierTo(Vec2, Vec2, Vec2),
}
