use crate::nodes::{CyberNodes, ItemMetaBundle, PortMeta, ValType};
use crate::{ui::UIRoot, Debug};
use crate::{Gamestate, ShoppingForTile};
use bevy::{
    color::palettes::css::BLACK,
    prelude::*,
    ui::{ui_layout_system, RelativeCursorPosition},
};
use hexx::{shapes, storage::HexagonalMap, Hex, HexLayout};

pub struct ShopPlugin;
impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_shop)
            .add_systems(
                Update,
                (align_grid, leave_shop, grid_selection, pick_grid)
                    .after(ui_layout_system)
                    .run_if(in_state(Gamestate::Shop)),
            )
            .add_event::<PickedItem>()
            .add_systems(OnEnter(Gamestate::Shop), show_shop)
            .add_systems(OnExit(Gamestate::Shop), hide_shop);
    }
}

#[derive(Event)]
pub struct PickedItem {
    pub item: Entity,
    pub tile: Hex,
}

#[derive(Resource)]
struct ShopUINode(Entity);

#[derive(Component)]
struct ShopTile;

#[derive(Component)]
struct ShopGrid;

#[derive(Component)]
struct ShopLayout(HexLayout);

#[derive(Component)]
struct ShopItems(HexagonalMap<Entity>);

#[derive(Component, Default)]
struct ShopSelection {
    tile: Option<Hex>,
    mouse: Option<Vec2>,
    node: Option<Entity>,
}

impl ShopItems {
    pub fn tiles(&self) -> impl Iterator<Item = (Hex, Entity)> + '_ {
        let bounds = self.0.bounds();
        shapes::hexagon(bounds.center, bounds.radius).map(|h| (h, *self.0.get(h).unwrap()))
    }
}

fn show_shop(node: Res<ShopUINode>, mut vis: Query<&mut Visibility>) {
    let mut v = vis
        .get_mut(node.0)
        .expect("shop ui has not been spawned yet");

    *v = Visibility::Visible;
}

fn hide_shop(
    node: Res<ShopUINode>,
    mut vis: Query<&mut Visibility>,
    mut fortile: ResMut<ShoppingForTile>,
) {
    fortile.0 = None;
    let mut v = vis
        .get_mut(node.0)
        .expect("shop ui has not been spawned yet");

    *v = Visibility::Hidden;
}

fn spawn_shop(
    mut cmd: Commands,
    root: Res<UIRoot>,
    ass: Res<AssetServer>,
    mut mats: ResMut<Assets<ColorMaterial>>,
) {
    let layout = HexLayout {
        hex_size: Vec2::splat(10.),
        ..default()
    };

    let empty_out = cmd
        .spawn(PortMeta::new_meta(
            "nothing".to_string(),
            "nothing".to_string(),
            ValType::Empty,
            false,
        ))
        .id();

    let mut items = [
        ItemMetaBundle::new(
            "lazor".to_string(),
            "shoots lazor beam at target".to_string(),
            &[cmd
                .spawn(PortMeta::new_meta(
                    "target".to_string(),
                    "the target entity to shoot".to_string(),
                    ValType::Entity,
                    false,
                ))
                .id()],
            empty_out,
            ass.load("nodes/lazor.png"),
            CyberNodes::Lazor,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "closest".to_string(),
            "returns the closest nearby entity".to_string(),
            &[],
            cmd.spawn(PortMeta::new_meta(
                "closest".to_string(),
                "the closest nearby entity".to_string(),
                ValType::Entity,
                false,
            ))
            .id(),
            ass.load("nodes/closest.png"),
            CyberNodes::ClosestEntity,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "constant number".to_string(),
            "returns a constant number set in the port config".to_string(),
            &[],
            cmd.spawn(PortMeta::new_meta(
                "constant".to_string(),
                "the constant value".to_string(),
                ValType::Number,
                true,
            ))
            .id(),
            ass.load("nodes/port_in.png"),
            CyberNodes::ConstantNumber,
            &mut mats,
        ),
    ]
    .into_iter();

    let emptyimg = ass.load("nodes/empty.png");
    let empty = ItemMetaBundle::new(
        "empty slot".to_string(),
        "wip".to_string(),
        &[],
        empty_out,
        emptyimg.clone(),
        CyberNodes::WIP,
        &mut mats,
    );

    let storage = HexagonalMap::new(Hex::ZERO, 5, |_| {
        let item = items.next().unwrap_or(empty.clone());
        let tex = item.tex.clone();
        cmd.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ..default()
            },
            ShopTile,
            item,
        ))
        .with_children(|node| {
            node.spawn((ImageBundle {
                style: Style {
                    width: Val::Auto,
                    height: Val::Auto,
                    ..default()
                },
                image: UiImage::new(tex),
                ..default()
            },));
        })
        .id()
    });

    let mut shop_box = cmd.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(90.),
            height: Val::Percent(90.),
            position_type: PositionType::Absolute,
            left: Val::Percent(5.),
            top: Val::Percent(5.),
            display: Display::Flex,
            justify_content: JustifyContent::FlexStart,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        background_color: Color::from(BLACK.with_alpha(0.7)).into(),
        visibility: Visibility::Hidden,
        ..default()
    });

    shop_box.with_children(|root| {
        // title
        root.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                ..default()
            },
            ..default()
        })
        .with_children(|titlebox| {
            titlebox.spawn(TextBundle::from_section(
                "Shop",
                TextStyle {
                    font: ass.load("fonts/Exwayer-X3eqa.ttf"),
                    font_size: 62.,
                    ..default()
                },
            ));
        });

        // main area
        root.spawn(NodeBundle {
            style: Style {
                //height: Val::Auto,
                //width: Val::Auto,
                flex_grow: 1.,
                ..default()
            },
            ..default()
        })
        .with_children(|main| {
            main.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.),
                    height: Val::Auto,
                    max_height: Val::Percent(100.),
                    aspect_ratio: Some(1.),
                    ..default()
                },
                ..default()
            })
            .with_children(|intermediate| {
                let tiles: Vec<Entity> = storage.iter().flatten().copied().collect();
                intermediate
                    .spawn((
                        NodeBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                width: Val::Auto,
                                height: Val::Percent(100.),
                                aspect_ratio: Some(1.),
                                ..default()
                            },
                            ..default()
                        },
                        ShopGrid,
                        ShopLayout(layout),
                        ShopSelection::default(),
                        RelativeCursorPosition::default(),
                        ShopItems(storage),
                    ))
                    .push_children(&tiles);
            });
        });
    });

    let e = shop_box.id();
    cmd.entity(root.0).add_child(e);
    cmd.insert_resource(ShopUINode(e));
}

fn grid_selection(
    mut shop: Query<(
        &ShopLayout,
        &ShopItems,
        &mut ShopSelection,
        &RelativeCursorPosition,
        &Node,
    )>,
    mut dbg: ResMut<Debug>,
) {
    for (layout, items, mut selection, rel, node) in shop.iter_mut() {
        selection.tile = None;
        selection.mouse = None;
        selection.node = None;
        if !rel.mouse_over() {
            continue;
        };

        let size = node.size();
        let half = size / 2.;

        let Some(rel) = rel.normalized else { continue };
        // denormalize and center cursor pos
        let pos = size * rel - half;
        dbg.mouse_shop_logical = pos;

        let tile = layout.0.world_pos_to_hex(pos);
        dbg.mouse_shop_tile = tile.as_ivec2();
        selection.mouse = Some(pos);
        if !items.0.bounds().is_in_bounds(tile) {
            return;
        };
        selection.node = Some(*items.0.get(tile).unwrap());
        selection.tile = Some(tile);
    }
}

fn pick_grid(
    mouse: Res<ButtonInput<MouseButton>>,
    shoppingfor: Res<ShoppingForTile>,
    selection: Query<&ShopSelection>,
    mut evt: EventWriter<PickedItem>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    for s in selection.iter() {
        let Some(node) = s.node else { continue };
        evt.send(PickedItem {
            item: node,
            tile: shoppingfor.0.expect("shop open without shoppingfor"),
        });
    }
}

fn align_grid(
    mut q: Query<&mut Style, With<ShopTile>>,
    mut grid: Query<(&Node, &mut ShopLayout, &ShopItems), With<ShopGrid>>,
) {
    let Ok((grid, mut layout, items)) = grid.get_single_mut() else {
        return;
    };

    let size = grid.size();
    let unit = size / 25.;
    let half = size / 2.;

    *layout = ShopLayout(HexLayout {
        hex_size: unit,
        ..default()
    });

    for (h, e) in items.tiles() {
        let mut tile = q.get_mut(e).unwrap();
        let pos = layout.0.hex_to_world_pos(h);
        let half_tile = layout.0.rect_size() / 2.;
        tile.left = Val::Px(pos.x + half.x - half_tile.x);
        tile.top = Val::Px(pos.y + half.y - half_tile.y);
        tile.width = Val::Px(layout.0.rect_size().x);
        tile.height = Val::Px(layout.0.rect_size().y);
        tile.padding = UiRect::all(Val::Px(2.));
    }
}

fn leave_shop(
    input: Res<ButtonInput<KeyCode>>,
    picked: EventReader<PickedItem>,
    mut shoppingfor: ResMut<ShoppingForTile>,
    mut state: ResMut<NextState<Gamestate>>,
) {
    if input.just_pressed(KeyCode::Escape) || !picked.is_empty() {
        info!("returning to Gamestate::Game");
        state.set(Gamestate::Game);
        shoppingfor.0 = None;
    }
}
