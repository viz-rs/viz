//! Tests how different transforms behave when clipped with `Overflow::Hidden`

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::{
    camera_controller::pan_camera::{PanCamera, PanCameraPlugin},
    input::common_conditions::input_just_pressed,
    prelude::*,
    text::TextWriter,
};
use taffy::prelude::*;

use bevy_moon::{
    MoonPlugin,
    elements::{image::ImageNode, node::Node, text::Text},
    style::Style,
};

mod utils;

use utils::on_drag_and_drop;

const CONTAINER_SIZE: f32 = 150.0;
const LOOP_LENGTH: f32 = 4.0;

#[derive(Component)]
struct Instructions;

#[derive(Resource, Default)]
struct AnimationState {
    playing: bool,
    paused_at: f32,
    paused_total: f32,
    t: f32,
}

#[derive(Component)]
struct Container(u8);

trait UpdateTransform {
    fn update(&self, t: f32, transform: &mut Transform);
}

#[derive(Component)]
struct Move;

impl UpdateTransform for Move {
    fn update(&self, t: f32, transform: &mut Transform) {
        transform.translation.x = ops::sin(t * TAU - FRAC_PI_2) * 50.0;
        transform.translation.y = ops::cos(t * TAU - FRAC_PI_2) * 50.0;
    }
}

#[derive(Component)]
struct Scale;

impl UpdateTransform for Scale {
    fn update(&self, t: f32, transform: &mut Transform) {
        transform.scale.x = 1.0 + 0.5 * ops::cos(t * TAU).max(0.0);
        transform.scale.y = 1.0 + 1.5 * ops::cos(t * TAU + PI).max(0.0);
    }
}

#[derive(Component)]
struct Rotate;

impl UpdateTransform for Rotate {
    fn update(&self, t: f32, transform: &mut Transform) {
        let q = Quat::from_rotation_z(ops::cos(t * TAU) * 45.0);
        transform.rotation = q;
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera

    commands.spawn((Camera2d, PanCamera::default()));

    // Instructions

    let text_font = TextFont::default();

    commands
        .spawn((
            Style::from(taffy::Style {
                position: taffy::Position::Absolute,
                inset: taffy::Rect {
                    top: taffy::LengthPercentageAuto::length(12.0),
                    left: taffy::LengthPercentageAuto::length(12.0),
                    right: taffy::LengthPercentageAuto::auto(),
                    bottom: taffy::LengthPercentageAuto::auto(),
                },
                ..default()
            }),
            Text::new(
                r#"Next Overflow Setting (O)
Next Container Size (P)
Next Flip Setting (F)
Toggle Animation (space)"#,
            ),
            text_font.clone(),
            Instructions,
            Transform::from_xyz(-400.0, 300.0, 0.0),
        ))
        .observe(on_drag_and_drop)
        .with_child((
            TextSpan::new(format!(
                "\nOverflow {{ x: {:?}, y: {:?} }}",
                taffy::Overflow::Clip,
                taffy::Overflow::Clip
            )),
            text_font.clone(),
        ))
        .with_child((
            TextSpan::new(format!("\nFlip Image {{ x: {:?}, y: {:?} }}", false, false)),
            text_font.clone(),
        ));

    // Overflow Debug

    commands
        .spawn((
            Node,
            Style::from(taffy::Style {
                justify_content: Some(taffy::JustifyContent::Center),
                align_items: Some(taffy::AlignItems::Center),
                ..default()
            }),
        ))
        .observe(on_drag_and_drop)
        .with_children(|parent| {
            parent
                .spawn((
                    Node,
                    Style::from(taffy::Style {
                        display: taffy::Display::Grid,
                        grid_template_columns: vec![length(CONTAINER_SIZE); 3],
                        grid_template_rows: vec![length(CONTAINER_SIZE); 2],
                        gap: taffy::Size {
                            width: taffy::LengthPercentage::length(80.0),
                            height: taffy::LengthPercentage::length(80.0),
                        },
                        ..default()
                    }),
                ))
                .with_children(|parent| {
                    spawn_image(parent, &asset_server, Move);
                    spawn_image(parent, &asset_server, Scale);
                    spawn_image(parent, &asset_server, Rotate);

                    spawn_text(parent, &asset_server, Move);
                    spawn_text(parent, &asset_server, Scale);
                    spawn_text(parent, &asset_server, Rotate);
                });
        });
}

fn spawn_image(
    parent: &mut ChildSpawnerCommands,
    asset_server: &Res<AssetServer>,
    update_transform: impl UpdateTransform + Component,
) {
    spawn_container(parent, update_transform, |parent| {
        parent.spawn((
            ImageNode::new(asset_server.load("images/bevy_logo_dark_big.png")),
            Style::from(taffy::Style {
                size: taffy::Size {
                    width: taffy::Dimension::auto(),
                    height: taffy::Dimension::length(100.0),
                },
                position: taffy::Position::Absolute,
                inset: taffy::Rect {
                    top: taffy::LengthPercentageAuto::length(-50.0),
                    left: taffy::LengthPercentageAuto::length(-200.0),
                    right: taffy::LengthPercentageAuto::auto(),
                    bottom: taffy::LengthPercentageAuto::auto(),
                },
                ..default()
            }),
        ));
    });
}

fn spawn_text(
    parent: &mut ChildSpawnerCommands,
    asset_server: &Res<AssetServer>,
    update_transform: impl UpdateTransform + Component,
) {
    spawn_container(parent, update_transform, |parent| {
        parent.spawn((
            Text::new("Bevy"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 100.0,
                ..default()
            },
        ));
    });
}

fn spawn_container(
    parent: &mut ChildSpawnerCommands,
    update_transform: impl UpdateTransform + Component,
    spawn_children: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((
            Node,
            Style::from(taffy::Style {
                size: taffy::Size::percent(1.0),
                align_items: Some(taffy::AlignItems::Center),
                justify_content: Some(taffy::JustifyContent::Center),
                overflow: taffy::Point {
                    x: taffy::Overflow::Clip,
                    y: taffy::Overflow::Clip,
                },
                ..default()
            })
            .background(Color::srgb(0.25, 0.25, 0.25)),
            Container(0),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node,
                    Style::from(taffy::Style {
                        align_items: Some(taffy::AlignItems::Center),
                        justify_content: Some(taffy::JustifyContent::Center),
                        ..default()
                    }),
                    update_transform,
                ))
                .with_children(spawn_children);
        });
}

fn update_animation(
    mut animation: ResMut<AnimationState>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let delta = time.elapsed_secs();

    if keys.just_pressed(KeyCode::Space) {
        animation.playing = !animation.playing;

        if !animation.playing {
            animation.paused_at = delta;
        } else {
            animation.paused_total += delta - animation.paused_at;
        }
    }

    if animation.playing {
        animation.t = (delta - animation.paused_total) % LOOP_LENGTH / LOOP_LENGTH;
    }
}

fn update_transform<T: UpdateTransform + Component>(
    animation: Res<AnimationState>,
    mut containers: Query<(&mut Transform, &T)>,
) {
    for (mut transform, update_transform) in &mut containers {
        update_transform.update(animation.t, &mut transform);
    }
}

fn toggle_flip(
    mut containers: Query<&mut ImageNode>,
    instructions: Single<Entity, With<Instructions>>,
    mut writer: TextWriter<Text>,
) {
    for mut image in &mut containers {
        let (x, y) = match (image.flip_x, image.flip_y) {
            (false, false) => (true, false),
            (true, false) => (true, true),
            (true, true) => (false, true),
            (false, true) => (false, false),
        };
        image.flip_x = x;
        image.flip_y = y;

        let entity = *instructions;
        *writer.text(entity, 2) = format!("\nFlip Image {{ x: {:?}, y: {:?} }}", x, y);
    }
}

fn toggle_overflow(
    mut containers: Query<&mut Style, With<Container>>,
    instructions: Single<Entity, With<Instructions>>,
    mut writer: TextWriter<Text>,
) {
    for mut style in &mut containers {
        style.get_mut().overflow = match style.get_ref().overflow {
            taffy::Point {
                x: taffy::Overflow::Visible,
                y: taffy::Overflow::Visible,
            } => taffy::Point {
                x: taffy::Overflow::Visible,
                y: taffy::Overflow::Clip,
            },
            taffy::Point {
                x: taffy::Overflow::Visible,
                y: taffy::Overflow::Clip,
            } => taffy::Point {
                x: taffy::Overflow::Clip,
                y: taffy::Overflow::Visible,
            },
            taffy::Point {
                x: taffy::Overflow::Clip,
                y: taffy::Overflow::Visible,
            } => taffy::Point {
                x: taffy::Overflow::Clip,
                y: taffy::Overflow::Clip,
            },
            _ => taffy::Point {
                x: taffy::Overflow::Visible,
                y: taffy::Overflow::Visible,
            },
        };

        let entity = *instructions;
        let taffy::Point { x, y } = style.get_ref().overflow;
        *writer.text(entity, 1) = format!("\nOverflow {{ x: {:?}, y: {:?} }}", x, y);
    }
}

fn next_container_size(mut containers: Query<(&mut Style, &mut Container)>) {
    for (mut style, mut container) in &mut containers {
        container.0 = (container.0 + 1) % 3;

        let width = match container.0 {
            2 => 0.3,
            _ => 1.0,
        };
        let height = match container.0 {
            1 => 0.3,
            _ => 1.0,
        };

        style.get_mut().size = taffy::Size {
            width: taffy::Dimension::percent(width),
            height: taffy::Dimension::percent(height),
        };
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PanCameraPlugin)
        .add_plugins(MoonPlugin)
        .init_resource::<AnimationState>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                toggle_flip.run_if(input_just_pressed(KeyCode::KeyF)),
                toggle_overflow.run_if(input_just_pressed(KeyCode::KeyO)),
                next_container_size.run_if(input_just_pressed(KeyCode::KeyP)),
                update_transform::<Move>,
                update_transform::<Scale>,
                update_transform::<Rotate>,
                update_animation,
            ),
        )
        .run();
}
