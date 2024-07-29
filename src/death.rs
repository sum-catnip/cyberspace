use bevy::{
    color::palettes::css::{BLACK, GREEN, WHITE},
    prelude::*,
};

use crate::{Appstate, Gamestate};

pub struct DeathPlugin;
impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Appstate::Death), init);
        app.add_systems(OnExit(Appstate::Death), deinit);
        app.add_systems(Update, menu_action);
    }
}

#[derive(Resource)]
struct Screen(Entity);

#[derive(Component)]
enum ButtonAction {
    Restart,
}

fn init(mut cmd: Commands, ass: Res<AssetServer>) {
    let screen = cmd
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                position_type: PositionType::Absolute,
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            background_color: Color::from(GREEN.with_alpha(0.7)).into(),
            ..default()
        })
        .with_children(|main| {
            main.spawn(TextBundle::from_section(
                "Cyberspace",
                TextStyle {
                    font: ass.load("fonts/Geist-Regular.ttf"),
                    font_size: 200.,
                    color: WHITE.into(),
                    ..default()
                },
            ));
            main.spawn((ButtonBundle { ..default() }, ButtonAction::Restart))
                .with_children(|back| {
                    back.spawn(TextBundle::from_section(
                        "Play",
                        TextStyle {
                            font: ass.load("fonts/Geist-Regular.ttf"),
                            font_size: 24.,
                            ..default()
                        },
                    ));
                });
        })
        .id();

    cmd.insert_resource(Screen(screen));
}

fn menu_action(
    interaction: Query<(&Interaction, &ButtonAction), (Changed<Interaction>, With<Button>)>,
    mut state: ResMut<NextState<Appstate>>,
) {
    for (interaction, action) in interaction.iter() {
        if *interaction == Interaction::Pressed {
            match action {
                ButtonAction::Restart => state.set(Appstate::Game),
            }
        }
    }
}

fn deinit(mut cmd: Commands, screen: Res<Screen>) {
    cmd.entity(screen.0).despawn_recursive();
}
