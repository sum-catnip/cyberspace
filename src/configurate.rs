use core::panic;

use bevy::{color::palettes::css::BLACK, math::vec2, prelude::*, ui::RelativeCursorPosition};
use hexx::{storage::HexagonalMap, Hex, HexLayout};

use crate::{
    nodes::{PortCfg, PortMeta, PortMetas},
    ui::UIRoot,
    CommonResources, ConfiguringTile, Gamestate, Map, TileType,
};

pub struct ConfiguratePlugin;
impl Plugin for ConfiguratePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Gamestate::Configurate), build_ui)
            .add_systems(OnExit(Gamestate::Configurate), destroy_ui)
            .add_systems(
                Update,
                (esc_to_exit, grid_select, grid_pick).run_if(in_state(Gamestate::Configurate)),
            );
    }
}

#[derive(Component)]
struct CfgUi(HexLayout);

#[derive(Component)]
struct CfgGrid(Entity);

#[derive(Component)]
struct GridTiles(HexagonalMap<Entity>);

#[derive(Component)]
struct GridMeta(Entity);

#[derive(Component, Default)]
struct GridSelection {
    tile: Option<Hex>,
    ui: Option<Entity>,
}

fn grid_pick(
    input: Res<ButtonInput<MouseButton>>,
    grids: Query<(&CfgGrid, &GridSelection, &GridTiles, &GridMeta)>,
    mut portcfg: Query<&mut PortCfg>,
    common: Res<CommonResources>,
    mut img: Query<&mut UiImage>,
) {
    if !input.just_pressed(MouseButton::Left) {
        return;
    };

    for (grid, selection, tiles, meta) in grids.iter() {
        let Some(tile_hex) = selection.tile else {
            continue;
        };
        let Some(tile_ent) = selection.ui else {
            continue;
        };

        let mut cfg = portcfg.get_mut(grid.0).expect("node without portcfg");
        if let Some(h) = cfg.0.iter().find(|(_, e)| **e == meta.0).map(|(h, _)| *h) {
            // this metaport has already been asigned to to another tile
            cfg.0.remove(&h);
        }

        if cfg.0.get(&tile_hex).is_some() {
            warn!("tile already taken");
            continue;
        }
        cfg.0.insert(tile_hex, meta.0);

        // lets reset all tiles lol
        for e in tiles.0.iter().flatten().copied() {
            img.get_mut(e).unwrap().texture = common.blank_img.clone();
        }

        img.get_mut(tile_ent).unwrap().texture = common.port_in.clone();
    }
}

fn grid_select(
    layout: Query<&CfgUi>,
    mut grids: Query<(
        &Node,
        &mut GridSelection,
        &GridTiles,
        &RelativeCursorPosition,
    )>,
) {
    let layout = layout.single();
    for (node, mut selection, items, rel) in grids.iter_mut() {
        selection.tile = None;
        selection.ui = None;

        if !rel.mouse_over() {
            continue;
        };

        let size = node.size();
        let half = size / 2.;

        let Some(rel) = rel.normalized else { continue };
        // denormalize and center cursor pos
        let pos = size * rel - half;

        let tile = layout.0.world_pos_to_hex(pos);
        if !items.0.bounds().is_in_bounds(tile) {
            return;
        };
        selection.ui = Some(*items.0.get(tile).unwrap());
        selection.tile = Some(tile);
    }
}

fn spawn_row(
    cmd: &mut Commands,
    metas: &Query<&PortMeta>,
    meta_ent: Entity,
    res: &CommonResources,
    node: Entity,
    layout: &HexLayout,
) -> Entity {
    let size = vec2(150., 150.);
    let half_size = size / 2.;
    let tiles = HexagonalMap::new(Hex::ZERO, 1, |h| {
        let pos = layout.hex_to_world_pos(h);
        let ts = layout.rect_size();
        let hts = ts / 2.;
        cmd.spawn(ImageBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Px(ts.x),
                height: Val::Px(ts.y),
                top: Val::Px(pos.y + half_size.y - hts.y),
                left: Val::Px(pos.x + half_size.x - hts.x),
                margin: UiRect::all(Val::Px(2.)),
                ..default()
            },
            image: UiImage::new(res.blank_img.clone()),
            ..default()
        })
        .id()
    });

    let tile_ents: Vec<Entity> = tiles.iter().flatten().copied().collect();
    let meta = metas.get(meta_ent).unwrap();

    cmd.spawn(NodeBundle {
        style: Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            ..default()
        },
        ..default()
    })
    .with_children(|row| {
        // left side (text)
        row.spawn(NodeBundle {
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|ui| {
            ui.spawn(TextBundle::from_section(
                meta.name.clone(),
                TextStyle::default(),
            ));
            ui.spawn(TextBundle::from_section(
                meta.desc.clone(),
                TextStyle::default(),
            ));
            ui.spawn(TextBundle::from_section(
                meta.vt.to_string(),
                TextStyle::default(),
            ));
        });

        // right side (grid)
        row.spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(size.x),
                    height: Val::Px(size.y),
                    ..default()
                },
                ..default()
            },
            GridMeta(meta_ent),
            GridSelection::default(),
            CfgGrid(node),
            GridTiles(tiles),
            RelativeCursorPosition::default(),
        ))
        .push_children(&tile_ents);
    })
    .id()
}

fn build_ui(
    mut cmd: Commands,
    root: Res<UIRoot>,
    tile: Res<ConfiguringTile>,
    map: Res<Map>,
    tts: Query<&TileType>,
    ports: Query<&PortMetas>,
    metas: Query<&PortMeta>,
    res: Res<CommonResources>,
) {
    let tt = tts.get(map.fetch_panic(tile.0.unwrap())).unwrap();
    let TileType::CyberNode { meta, e, .. } = *tt else {
        panic!("configuration open for non cybernode");
    };
    let node = e;

    let layout = HexLayout {
        hex_size: Vec2::splat(20.),
        ..default()
    };

    let ports = ports.get(meta).expect("cybernode with no meta");

    let in_text = cmd
        .spawn(TextBundle::from_section(
            "input ports".to_string(),
            TextStyle::default(),
        ))
        .id();
    let in_rows: Vec<Entity> = ports
        .0
        .iter()
        .map(|e| spawn_row(&mut cmd, &metas, *e, &res, node, &layout))
        .collect();

    let ui = cmd
        .spawn((
            NodeBundle {
                style: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    width: Val::Auto,
                    height: Val::Auto,
                    ..default()
                },
                background_color: Color::from(BLACK.with_alpha(0.6)).into(),
                ..default()
            },
            CfgUi(layout),
        ))
        .add_child(in_text)
        .push_children(&in_rows)
        .id();

    cmd.entity(root.0).add_child(ui);
}

fn esc_to_exit(
    input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<NextState<Gamestate>>,
    mut configuring: ResMut<ConfiguringTile>,
) {
    if input.just_pressed(KeyCode::Escape) {
        info!("returning to Gamestate::Game");
        state.set(Gamestate::Game);
        configuring.0 = None;
    }
}

fn destroy_ui(mut cmd: Commands, cfg: Query<Entity, With<CfgUi>>) {
    for e in cfg.iter() {
        cmd.entity(e).despawn_recursive();
    }
}