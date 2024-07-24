mod configurate;
mod enemy;
mod nodes;
mod shop;
mod ui;
use std::time::Duration;

use enemy::EnemyPlugin;
use nodes::{
    ClosestEntity, ConstantNumber, CyberNodes, CyberPlugin, CyberState, EntityDirection, EntityPos,
    HexPos, Lazor, List, MetaLink, NearbyEntity, NodeBundle, NumberMul, NumberSub, Orbital, Plasma,
    PortCfg, PortMetas, Project, RocketLauncher, Shock, TickNode, Vector, VectorLen, VectorMul,
    VectorNeg,
};
use shop::PickedItem;
use ui::UIPlugin;

use bevy::{
    color::palettes::css::{GREEN, RED},
    core_pipeline::bloom::BloomSettings,
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use bevy_inspector_egui::{prelude::*, quick::ResourceInspectorPlugin};
use hexx::{storage::HexagonalMap, Hex, HexLayout, PlaneMeshBuilder};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_plugins(UIPlugin)
        .add_plugins(CyberPlugin)
        .add_plugins(EnemyPlugin)
        .add_plugins(ResourceInspectorPlugin::<Debug>::default())
        .add_systems(
            Update,
            (
                mouse_selection,
                click,
                open_shop,
                tile_purchased,
                remove_node,
                open_configurator,
                heartbeat,
                tick_nodes,
                request_nodes,
            )
                .run_if(in_state(Gamestate::Game)),
        )
        .add_event::<TileClicked>()
        .add_event::<Tick>()
        .init_resource::<Debug>()
        .init_resource::<ShoppingForTile>()
        .init_resource::<ConfiguringTile>()
        .init_resource::<Selection>()
        .init_state::<Gamestate>()
        .run();
}

const HEX_SIZE: Vec2 = Vec2::splat(25.);

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash, Reflect)]
enum Gamestate {
    #[default]
    Game,
    Shop,
    Configurate,
}

#[derive(Resource, Reflect, Default, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct Debug {
    mouse_logical: Vec2,
    mouse_world: Vec2,
    mouse_tile: IVec2,
    mouse_shop_logical: Vec2,
    mouse_shop_tile: IVec2,
    gui_outline: bool,
}

#[derive(Resource, Default)]
struct Selection {
    mouseover: Option<Hex>,
    mousepos: Option<Vec2>,
}

#[derive(Event)]
struct TileClicked {
    tile: Hex,
    mouse: Vec2,
    button: MouseButton,
}

#[derive(Event)]
struct Tick(Entity);

#[derive(Resource, Default)]
pub struct ShoppingForTile(Option<Hex>);

#[derive(Resource, Default)]
pub struct ConfiguringTile(Option<Hex>);

#[derive(Resource)]
struct Map {
    layout: HexLayout,
    storage: HexagonalMap<Entity>,
}

impl Map {
    fn fetch_panic(&self, h: Hex) -> Entity {
        *self
            .storage
            .get(h)
            .expect(&format!("tried to fetch oob tile: {:?}", h))
    }

    fn type_panic(&self, h: Hex, typeq: &Query<&TileType>) -> TileType {
        *typeq
            .get(self.fetch_panic(h))
            .expect(&format!("tile {:?} has no type!", h))
    }
}

#[derive(Component, Default, Clone, Copy, PartialEq, Eq)]
enum TileType {
    #[default]
    Unoccupied,
    Terrain,
    Heart(Entity),
    CyberNode {
        meta: Entity,
        e: Entity,
    },
}

#[derive(Component)]
struct Heartbeat(Timer);

#[derive(Bundle)]
struct HeartBundle {
    beat: Heartbeat,
    tile: HexPos,
}

type TileApperance = MaterialMesh2dBundle<ColorMaterial>;
#[derive(Bundle)]
struct HexTile {
    apperance: TileApperance,
    t: TileType,
}

#[derive(Component)]
struct MainCamera;

#[derive(Resource)]
struct CommonResources {
    unoccupied_mat: Handle<ColorMaterial>,
    blank_img: Handle<Image>,
    port_in: Handle<Image>,
    heart_mat: Handle<ColorMaterial>,
}

fn setup(
    mut cmd: Commands,
    mut mesh: ResMut<Assets<Mesh>>,
    mut mat: ResMut<Assets<ColorMaterial>>,
    ass: Res<AssetServer>,
) {
    cmd.spawn((
        Camera2dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::AcesFitted,
            ..default()
        },
        BloomSettings {
            intensity: 0.5,
            ..default()
        },
        MainCamera,
        IsDefaultUiCamera,
    ));

    let layout = HexLayout {
        hex_size: HEX_SIZE,
        ..default()
    };

    let tilemesh: Mesh2dHandle = mesh.add(hexmesh(&layout)).into();
    let red = mat.add(Color::from(RED));
    let heart_mat = mat.add(ass.load("nodes/heart.png"));

    cmd.insert_resource(CommonResources {
        unoccupied_mat: red.clone(),
        blank_img: ass.load("nodes/empty.png"),
        port_in: ass.load("nodes/port_in.png"),
        heart_mat: heart_mat.clone(),
    });

    let rad = 3;
    let storage = HexagonalMap::new(Hex::ZERO, rad, |t| {
        let worldpos = layout.hex_to_world_pos(t);
        let id = if t == Hex::ZERO {
            let e = cmd
                .spawn(HeartBundle {
                    beat: Heartbeat(Timer::new(Duration::from_secs(1), TimerMode::Repeating)),
                    tile: HexPos(t),
                })
                .id();
            cmd.spawn(HexTile {
                apperance: ColorMesh2dBundle {
                    transform: Transform::from_translation(worldpos.extend(0.)),
                    mesh: tilemesh.clone(),
                    material: heart_mat.clone(),
                    ..default()
                },
                t: TileType::Heart(e),
            })
            .id()
        } else {
            cmd.spawn(HexTile {
                apperance: ColorMesh2dBundle {
                    transform: Transform::from_translation(worldpos.extend(0.)),
                    mesh: tilemesh.clone(),
                    material: red.clone(),
                    ..default()
                },
                t: TileType::Unoccupied,
            })
            .id()
        };

        id
    });

    cmd.insert_resource(Map { layout, storage })
}

fn heartbeat(
    mut hearts: Query<(Entity, &mut Heartbeat)>,
    time: Res<Time>,
    mut tick: EventWriter<Tick>,
) {
    for (e, mut beat) in hearts.iter_mut() {
        if beat.0.tick(time.delta()).finished() {
            tick.send(Tick(e));
        }
    }
}

fn request_nodes(mut tick: EventReader<Tick>, mut nodes: Query<&mut CyberState>) {
    // todo: restrict this to nodes connected to the heart
    for _ in tick.read() {
        for mut state in nodes.iter_mut() {
            *state = CyberState::ActivationRequest;
        }
    }
}

fn tick_nodes(
    mut cmd: Commands,
    map: Res<Map>,
    tiles: Query<&TileType>,
    nodes: Query<(Entity, &CyberState, &PortCfg, &MetaLink, &HexPos)>,
    metas: Query<&CyberNodes>,
) {
    for (e, _, cfg, ml, hex) in nodes
        .iter()
        .filter(|(_, s, ..)| **s == CyberState::ActivationRequest)
    {
        let node = metas.get(ml.0).unwrap();

        // check if all port nodes are satisfied
        let satisfied = cfg.inputs.iter().all(|(ph, _)| {
            let Some(tile) = map.storage.get(hex.0 + *ph) else {
                return false;
            };

            let TileType::CyberNode { e, .. } = tiles.get(*tile).unwrap() else {
                return false;
            };
            let Ok((_, s, ..)) = nodes.get(*e) else {
                // not spawned yet
                return false;
            };
            matches!(*s, CyberState::Done(Ok(..)))
        });

        let node = *node;
        if satisfied {
            cmd.add(move |world: &mut World| {
                match node {
                    CyberNodes::WIP => (),
                    CyberNodes::Lazor => drop(world.send_event(TickNode::<Lazor>::new(e))),
                    CyberNodes::ClosestEntity => {
                        world.send_event(TickNode::<ClosestEntity>::new(e));
                    }
                    CyberNodes::ConstantNumber => {
                        world.send_event(TickNode::<ConstantNumber>::new(e));
                    }
                    CyberNodes::List => {
                        world.send_event(TickNode::<List>::new(e));
                    }
                    CyberNodes::EntityPos => {
                        world.send_event(TickNode::<EntityPos>::new(e));
                    }
                    CyberNodes::EntityDirection => {
                        world.send_event(TickNode::<EntityDirection>::new(e));
                    }
                    CyberNodes::NearbyEntities => {
                        world.send_event(TickNode::<NearbyEntity>::new(e));
                    }
                    CyberNodes::VectorLen => {
                        world.send_event(TickNode::<VectorLen>::new(e));
                    }
                    CyberNodes::VectorNeg => {
                        world.send_event(TickNode::<VectorNeg>::new(e));
                    }
                    CyberNodes::NumberSub => {
                        world.send_event(TickNode::<NumberSub>::new(e));
                    }
                    CyberNodes::NumberMul => {
                        world.send_event(TickNode::<NumberMul>::new(e));
                    }
                    CyberNodes::VectorMul => {
                        world.send_event(TickNode::<VectorMul>::new(e));
                    }
                    CyberNodes::Project => {
                        world.send_event(TickNode::<Project>::new(e));
                    }
                    CyberNodes::Shock => {
                        world.send_event(TickNode::<Shock>::new(e));
                    }
                    CyberNodes::Plasma => {
                        world.send_event(TickNode::<Plasma>::new(e));
                    }
                    CyberNodes::Orbital => {
                        world.send_event(TickNode::<Orbital>::new(e));
                    }
                    CyberNodes::RocketLauncher => {
                        world.send_event(TickNode::<RocketLauncher>::new(e));
                    }
                    CyberNodes::Vector => {
                        world.send_event(TickNode::<Vector>::new(e));
                    }
                };
            });
        }
    }
}

fn tile_purchased(
    mut cmd: Commands,
    mut evt: EventReader<PickedItem>,
    map: ResMut<Map>,
    mut mats: Query<&mut Handle<ColorMaterial>>,
    mut tiles: Query<&mut TileType>,
) {
    for item in evt.read() {
        let e = map.fetch_panic(item.tile);
        let nodemat = mats
            .get(item.item)
            .expect("purchased nonexistant item")
            .clone();

        let mut tm = mats.get_mut(e).expect("purchased item has no mat");
        let mut tt = tiles
            .get_mut(e)
            .expect("item purchased for non-existant tile");

        *tm = nodemat;
        *tt = TileType::CyberNode {
            meta: item.item,
            e: cmd
                .spawn(NodeBundle {
                    meta: MetaLink(item.item),
                    cfg: PortCfg::default(),
                    state: CyberState::Idle,
                    pos: HexPos(item.tile),
                })
                .id(),
        };
    }
}

fn remove_node(
    mut cmd: Commands,
    mut click: EventReader<TileClicked>,
    res: Res<CommonResources>,
    map: Res<Map>,
    mut tiles: Query<(&mut Handle<ColorMaterial>, &mut TileType)>,
) {
    for click in click.read().filter(|c| c.button == MouseButton::Right) {
        let (mut mat, mut tt) = tiles.get_mut(map.fetch_panic(click.tile)).unwrap();
        *mat = res.unoccupied_mat.clone();
        if let TileType::CyberNode { e, .. } = *tt {
            cmd.entity(e).despawn();
        }
        *tt = TileType::Unoccupied;
    }
}

fn click(
    selection: Res<Selection>,
    input: Res<ButtonInput<MouseButton>>,
    mut evt: EventWriter<TileClicked>,
) {
    if input.just_pressed(MouseButton::Left) || input.just_pressed(MouseButton::Right) {
        let Some(tile) = selection.mouseover else {
            // not over any tile
            return;
        };

        let Some(mouse) = selection.mousepos else {
            warn!(
                "mouse was hovering tile {:?}, but there was no worldpos",
                tile
            );
            return;
        };

        evt.send(TileClicked {
            tile,
            mouse,
            button: MouseButton::Left,
        });
    }
}

fn open_configurator(
    map: Res<Map>,
    typeq: Query<&TileType>,
    mut click: EventReader<TileClicked>,
    mut configuring: ResMut<ConfiguringTile>,
    mut state: ResMut<NextState<Gamestate>>,
) {
    for evt in click.read() {
        let tt = *typeq.get(map.fetch_panic(evt.tile)).unwrap();
        if evt.button == MouseButton::Left && matches!(tt, TileType::CyberNode { .. }) {
            configuring.0 = Some(evt.tile);
            state.set(Gamestate::Configurate);
        }
    }
}

fn open_shop(
    map: Res<Map>,
    typeq: Query<&TileType>,
    mut click: EventReader<TileClicked>,
    mut shopping: ResMut<ShoppingForTile>,
    mut state: ResMut<NextState<Gamestate>>,
) {
    for evt in click.read() {
        if evt.button == MouseButton::Left
            && *typeq.get(map.fetch_panic(evt.tile)).unwrap() == TileType::Unoccupied
        {
            state.set(Gamestate::Shop);
            shopping.0 = Some(evt.tile);
        }
    }
}

fn mouse_selection(
    mut dbg: ResMut<Debug>,
    mut selection: ResMut<Selection>,
    map: Res<Map>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    win: Query<&Window>,
) {
    selection.mouseover = None;

    let (cam, cam_trans) = cam.single();
    let win = win.single();
    let Some(cursor) = win.cursor_position() else {
        return;
    };

    let Some(worldpos) = cam.viewport_to_world_2d(cam_trans, cursor) else {
        return;
    };

    let tile = map.layout.world_pos_to_hex(worldpos);
    dbg.mouse_tile = tile.as_ivec2();
    dbg.mouse_world = worldpos;
    dbg.mouse_logical = cursor;
    if map.storage.bounds().is_in_bounds(tile) {
        selection.mouseover = Some(tile);
        selection.mousepos = Some(worldpos);
    }
}

fn hexmesh(layout: &HexLayout) -> Mesh {
    let plane = PlaneMeshBuilder::new(layout)
        .with_scale(Vec3::splat(0.9))
        .facing(Vec3::Z)
        .center_aligned()
        .build();

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, plane.vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, plane.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, plane.uvs)
    .with_inserted_indices(Indices::U16(plane.indices))
}
