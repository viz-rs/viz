use bevy::{
    camera_controller::pan_camera::{PanCamera, PanCameraPlugin},
    color::palettes::css::{ANTIQUE_WHITE, BLACK, GRAY},
    prelude::*,
};

use bevy_moon::{
    MoonPlugin,
    elements::{image::ImageNode, node::Node, text::Text},
    style::{BorderColor, Outline, Style},
};

mod utils;

use utils::on_drag_and_drop;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image = asset_server.load::<Image>("images/bevy.png");
    let text_style = TextFont::default();

    commands.spawn((Camera2d, PanCamera::default()));

    commands
        .spawn((
            Node,
            Style::from(taffy::Style {
                align_items: Some(taffy::AlignItems::Center),
                justify_content: Some(taffy::JustifyContent::Center),
                ..Default::default()
            })
            .background(Color::NONE),
        ))
        .observe(on_drag_and_drop)
        .with_children(|parent| {
            for (overflow_x, overflow_y, flip_x, flip_y) in [
                (
                    taffy::Overflow::Visible,
                    taffy::Overflow::Visible,
                    false,
                    false,
                ),
                (taffy::Overflow::Clip, taffy::Overflow::Visible, true, false),
                (taffy::Overflow::Visible, taffy::Overflow::Clip, false, true),
                (taffy::Overflow::Clip, taffy::Overflow::Clip, true, true),
            ] {
                let overflow = taffy::Point {
                    x: overflow_x,
                    y: overflow_y,
                };

                let mut image_node = ImageNode::new(image.clone());
                if flip_x {
                    image_node = image_node.with_flip_x();
                }
                if flip_y {
                    image_node = image_node.with_flip_y();
                }

                parent
                    .spawn((
                        Node,
                        Style::from(taffy::Style {
                            flex_direction: taffy::FlexDirection::Column,
                            align_items: Some(taffy::AlignItems::Center),
                            margin: taffy::Rect {
                                left: taffy::LengthPercentageAuto::length(25.0),
                                top: taffy::LengthPercentageAuto::auto(),
                                right: taffy::LengthPercentageAuto::length(25.0),
                                bottom: taffy::LengthPercentageAuto::auto(),
                            },
                            ..default()
                        })
                        .background(ANTIQUE_WHITE),
                    ))
                    .with_children(|parent| {
                        let label =
                            format!("overflow-x:{overflow_x:#?}\noverflow-y:{overflow_y:#?}\nflip_x:{flip_x:#?}\nflip_y:{flip_y:#?}");
                        parent
                            .spawn((
                                Node,
                                Style::from(taffy::Style {
                                    flex_direction: taffy::FlexDirection::Column,
                                    align_items: Some(taffy::AlignItems::Center),
                                    padding: taffy::Rect::length(10.0),
                                    margin: taffy::Rect {
                                        left: taffy::LengthPercentageAuto::auto(),
                                        top: taffy::LengthPercentageAuto::auto(),
                                        right: taffy::LengthPercentageAuto::auto(),
                                        bottom: taffy::LengthPercentageAuto::length(25.0),
                                    },
                                    ..default()
                                })
                                .background(Color::srgb(0.25, 0.25, 0.25)),
                            ))
                            .with_children(|parent| {
                                parent.spawn((Text::new(label), text_style.clone()));
                            });

                        parent.spawn((
                            Node,
                            Style::from(taffy::Style {
                                size: taffy::Size::length(100.0),
                                overflow,
                                padding: taffy::Rect {
                                    left: taffy::LengthPercentage::length(25.0),
                                    top: taffy::LengthPercentage::length(25.0),
                                    right: taffy::LengthPercentage::length(0.0),
                                    bottom: taffy::LengthPercentage::length(0.0),
                                },
                                margin: taffy::Rect {
                                    left: taffy::LengthPercentageAuto::length(25.0),
                                    top: taffy::LengthPercentageAuto::auto(),
                                    right: taffy::LengthPercentageAuto::length(25.0),
                                    bottom: taffy::LengthPercentageAuto::auto(),
                                },
                                border: taffy::Rect::length(5.0),
                                ..default()
                            })
                            .border_color(BorderColor::all(Color::BLACK))
                            .background(GRAY),
                            children![(
                                Node,
                                Style::from(taffy::Style {
                                    min_size: taffy::Size::length(100.0),
                                    ..default()
                                })
                                .outline(Outline {
                                    width: taffy::LengthPercentage::length(2.0),
                                    offset: taffy::LengthPercentage::length(2.0),
                                    color: Color::NONE,
                                })
                                .background(BLACK.with_alpha(0.5)),
                                image_node,
                            )],
                        ));

                    });
            }
        });
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
