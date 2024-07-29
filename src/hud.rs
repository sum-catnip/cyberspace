use bevy::prelude::*;

use crate::{nodes::Health, ui::UIRoot, Gamestate, Heartbeat, Map, Selection};

pub struct HudPlugin;
impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(OnEnter(Gamestate::Game), show)
            .add_systems(OnExit(Gamestate::Game), hide)
            .add_systems(
                FixedUpdate,
                (update_cycles, update_energy, update_pos, update_tile)
                    .run_if(in_state(Gamestate::Game)),
            );
    }
}

#[derive(Resource)]
struct UI(Entity);

#[derive(Component)]
struct CpuEnergy;

#[derive(Component)]
struct CpuCycles;

#[derive(Component)]
struct HoverPos;

#[derive(Component)]
struct HoverTile;

fn setup(mut cmd: Commands, root: Res<UIRoot>, ass: Res<AssetServer>) {
    let textstyle = TextStyle {
        font: ass.load("fonts/Geist-Regular.ttf"),
        font_size: 24.,
        ..default()
    };

    let ui = cmd
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(10.),
                top: Val::Px(10.),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            visibility: Visibility::Hidden,
            ..default()
        })
        .with_children(|hud| {
            hud.spawn((
                TextBundle::from_sections([
                    TextSection::new("cpu energy: ", textstyle.clone()),
                    TextSection::new("", textstyle.clone()),
                ]),
                CpuEnergy,
            ));

            hud.spawn((
                TextBundle::from_sections([
                    TextSection::new("cycles: ", textstyle.clone()),
                    TextSection::new("", textstyle.clone()),
                ]),
                CpuCycles,
            ));

            hud.spawn((
                TextBundle::from_sections([
                    TextSection::new("pos: ", textstyle.clone()),
                    TextSection::new("", textstyle.clone()),
                ]),
                HoverPos,
            ));

            hud.spawn((
                TextBundle::from_sections([
                    TextSection::new("tile: ", textstyle.clone()),
                    TextSection::new("", textstyle.clone()),
                ]),
                HoverTile,
            ));
        })
        .id();

    cmd.entity(root.0).add_child(ui);
    cmd.insert_resource(UI(ui));
}

fn update_energy(mut text: Query<&mut Text, With<CpuEnergy>>, hp: Query<&Health, With<Heartbeat>>) {
    let hp = hp.single();
    for mut text in text.iter_mut() {
        text.sections[1].value = hp.0.to_string();
    }
}

fn update_cycles(mut text: Query<&mut Text, With<CpuCycles>>, hp: Query<&Heartbeat>) {
    let beat = hp.single();
    for mut text in text.iter_mut() {
        text.sections[1].value = (1. / beat.0.duration().as_secs_f32()).to_string();
    }
}

fn update_tile(mut text: Query<&mut Text, With<HoverTile>>, selection: Res<Selection>) {
    for mut text in text.iter_mut() {
        if let Some(mouse) = selection.mouseover {
            text.sections[1].value = mouse.as_ivec2().to_string();
        }
    }
}

fn update_pos(
    mut text: Query<&mut Text, With<HoverPos>>,
    selection: Res<Selection>,
    map: Res<Map>,
) {
    for mut text in text.iter_mut() {
        if let Some(mouse) = selection.mousepos {
            let tile = map.layout.world_pos_to_fract_hex(mouse);
            text.sections[1].value = tile.to_string();
        }
    }
}

fn show(ui: Res<UI>, mut vis: Query<&mut Visibility>) {
    *vis.get_mut(ui.0).unwrap() = Visibility::Visible;
}

fn hide(ui: Res<UI>, mut vis: Query<&mut Visibility>) {
    *vis.get_mut(ui.0).unwrap() = Visibility::Hidden;
}
