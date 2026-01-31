use bevy::{
    camera_controller::pan_camera::{PanCamera, PanCameraPlugin},
    color::palettes::css::{ANTIQUE_WHITE, BLACK, GRAY},
    prelude::*,
};

use bevy_moon::{
    MoonPlugin,
    elements::{node::Node, text::Text},
    properties::overflow::OverflowClipMargin,
    style::{BorderColor, Style},
};

mod utils;

use utils::on_drag_and_drop;

const LIPSUM: &str = include_str!("../assets/lispsum.txt");

fn setup(mut commands: Commands) {
    let text_style = TextFont::default();

    commands.spawn((Camera2d, PanCamera::default()));

    for (i, &(transform, (width, height), (overflow_x, overflow_y), justify)) in [
        (
            Transform::from_xyz(-350.0, 0.0, 0.0),
            (taffy::Dimension::length(300.0), taffy::Dimension::auto()),
            (taffy::Overflow::Visible, taffy::Overflow::Visible),
            Justify::Left,
        ),
        (
            Transform::from_xyz(0.0, 150.0, 0.0),
            (
                taffy::Dimension::length(300.0),
                taffy::Dimension::length(350.0),
            ),
            (taffy::Overflow::Clip, taffy::Overflow::Visible),
            Justify::Center,
        ),
        (
            Transform::from_xyz(0.0, -150.0, 0.0),
            (taffy::Dimension::auto(), taffy::Dimension::auto()),
            (taffy::Overflow::Visible, taffy::Overflow::Clip),
            Justify::Right,
        ),
        (
            Transform::from_xyz(350.0, 150.0, 0.0),
            (
                taffy::Dimension::length(300.0),
                taffy::Dimension::length(350.0),
            ),
            (taffy::Overflow::Clip, taffy::Overflow::Clip),
            Justify::Justified,
        ),
    ]
    .iter()
    .enumerate()
    {
        let overflow = taffy::Point {
            x: overflow_x,
            y: overflow_y,
        };

        commands
            .spawn((
                transform,
                Node,
                Style::from(taffy::Style {
                    display: taffy::Display::Flex,
                    flex_direction: taffy::FlexDirection::Column,
                    padding: taffy::Rect::length(10.0),
                    border: taffy::Rect::length(2.0),
                    size: taffy::Size {
                        width,
                        height,
                    },
                    max_size: taffy::Size {
                      width: taffy::Dimension::length(500.0),
                      height: taffy::Dimension::auto(),
                    },
                    overflow,
                    ..default()
                })
                .background(ANTIQUE_WHITE)
                .border_color(BorderColor::all(GRAY.with_alpha(0.5)))
                .overflow_clip_margin(OverflowClipMargin::content_box()),
            ))
            .observe(on_drag_and_drop)
            .with_children(|parent| {
                let badge = format!("{}", i + 1);
                let label = format!(
                    "overflow-x:{overflow_x:#?}\noverflow-y:{overflow_y:#?}\ntext-align:{justify:#?}"
                );

                parent
                    .spawn((
                        Node,
                        Style::from(taffy::Style {
                          size: taffy::Size {
                            width: taffy::Dimension::auto(),
                            height: taffy::Dimension::auto(),
                          },
                          padding: taffy::Rect::length(5.0),
                          ..default()
                        })
                        .background(BLACK.with_alpha(0.8765)),
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Style::from(taffy::Style {
                                display: taffy::Display::Block,
                                position: taffy::Position::Absolute,
                                inset: taffy::Rect {
                                  top: taffy::LengthPercentageAuto::length(0.0),
                                  right: taffy::LengthPercentageAuto::length(0.0),
                                  left: taffy::LengthPercentageAuto::auto(),
                                  bottom: taffy::LengthPercentageAuto::auto(),
                                },
                                ..default()
                            }).background(BLACK),
                            Text::new(badge),
                            text_style.clone(),
                        ));

                        parent.spawn((
                            Text::new(label),
                            text_style.clone(),
                        ));
                    });

                parent.spawn((
                    Style::from_taffy(taffy::Style {
                        size: taffy::Size {
                            width: taffy::Dimension::percent(1.0),
                            height: taffy::Dimension::auto(),
                        },
                        ..default()
                    }),
                    Text::new(LIPSUM),
                    TextColor(BLACK.into()),
                    TextLayout {
                        justify,
                        ..default()
                    },
                    text_style.clone(),
                ));
            });
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(PanCameraPlugin)
        .add_plugins(MoonPlugin)
        .add_systems(Startup, setup)
        .insert_resource(ClearColor(Color::oklch(0.98, 0.0, 0.0)));

    app.run();
}
