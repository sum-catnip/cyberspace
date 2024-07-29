use crate::nodes::{
    CyberNodes, Description, ItemMeta, ItemMetaBundle, Name, OutputPort, PortMeta, PortMetas,
    ValType,
};
use crate::{ui::UIRoot, Debug};
use crate::{Gamestate, ShoppingForTile};
use bevy::{color::palettes::css::BLACK, prelude::*, ui::RelativeCursorPosition};
use hexx::{shapes, storage::HexagonalMap, Hex, HexLayout};

pub struct ShopPlugin;
impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_shop)
            .add_systems(
                Update,
                (
                    align_grid,
                    leave_shop,
                    grid_selection,
                    pick_grid,
                    update_description,
                )
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
struct ItemDescription;

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
            "rocket launcher".to_string(),
            "shoots rockets in the target direction".to_string(),
            &[cmd
                .spawn(PortMeta::new_meta(
                    "direction".to_string(),
                    "the direction to shoot in".to_string(),
                    ValType::Vec,
                    false,
                ))
                .id()],
            empty_out,
            ass.load("nodes/launcher.png"),
            CyberNodes::RocketLauncher,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "orbital strike".to_string(),
            "request an orbital strike at a position that will arrive in the future".to_string(),
            &[cmd
                .spawn(PortMeta::new_meta(
                    "target".to_string(),
                    "the target position".to_string(),
                    ValType::Vec,
                    false,
                ))
                .id()],
            empty_out,
            ass.load("nodes/orbital.png"),
            CyberNodes::Orbital,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "plasma cannon".to_string(),
            "shoot a plasma to the target position. the size of the plasma depends on how many ticks the plasma cannon has been charged".to_string(),
            &[cmd
                .spawn(PortMeta::new_meta(
                    "target".to_string(),
                    "the target position".to_string(),
                    ValType::Vec,
                    false,
                )).id(),
                cmd.spawn(PortMeta::new_meta(
                    "threshold".to_string(),
                    "the amount of ticks to collect before firing. default is max: 10".to_string(),
                    ValType::Number,
                    false,
                ))
                .id()],
            cmd.spawn(PortMeta::new_meta(
                "fired".to_string(),
                "power of the shot or 0 if it didnt fire this tick".to_string(),
                ValType::Number,
                false,
            ))
            .id(),
            ass.load("nodes/plasma.png"),
            CyberNodes::Plasma,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "tesla coil".to_string(),
            "shoot lightning at all targets".to_string(),
            &[cmd
                .spawn(PortMeta::new_meta(
                    "targets".to_string(),
                    "list of target entities".to_string(),
                    ValType::List,
                    false,
                )).id(),
                ],
            cmd.spawn(PortMeta::new_meta(
                "shot".to_string(),
                "number of targets shot".to_string(),
                ValType::Number,
                false,
            ))
            .id(),
            ass.load("nodes/shock.png"),
            CyberNodes::Shock,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "project tile".to_string(),
            "project an illusory tile at the target position".to_string(),
            &[cmd
                .spawn(PortMeta::new_meta(
                    "target".to_string(),
                    "target position".to_string(),
                    ValType::Vec,
                    false,
                )).id(),
                ],
            empty_out,
            ass.load("nodes/project_tile.png"),
            CyberNodes::Project,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "debug".to_string(),
            "log all inputs to the console".to_string(),
            &[
                cmd.spawn(PortMeta::new_meta(
                    "a".to_string(),
                    "first item".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "b".to_string(),
                    "second item".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "c".to_string(),
                    "third item".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "d".to_string(),
                    "fourth item".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "e".to_string(),
                    "fifth item".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
            ],
            empty_out,
            ass.load("nodes/debug.png"),
            CyberNodes::Debug,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "list: construct".to_string(),
            "construct list out of all inputs, input lists will be flattened".to_string(),
            &[
                cmd.spawn(PortMeta::new_meta(
                    "a".to_string(),
                    "first item".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "b".to_string(),
                    "second item".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "c".to_string(),
                    "third item".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "d".to_string(),
                    "fourth item".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "e".to_string(),
                    "fifth item".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
            ],
            cmd.spawn(PortMeta::new_meta(
                "list".to_string(),
                "1 dimentional list of all inputs".to_string(),
                ValType::List,
                false,
            ))
            .id(),
            ass.load("nodes/list.png"),
            CyberNodes::List,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "entity: closest".to_string(),
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
            "number: constant".to_string(),
            "returns a constant number set in the port config".to_string(),
            &[],
            cmd.spawn(PortMeta::new_meta(
                "constant".to_string(),
                "the constant value".to_string(),
                ValType::Number,
                true,
            ))
            .id(),
            ass.load("nodes/const_number.png"),
            CyberNodes::ConstantNumber,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "vector: multiply".to_string(),
            "multilies an arbitrary amount of vectors".to_string(),
            &[
                cmd.spawn(PortMeta::new_meta(
                    "a".to_string(),
                    "first vector".to_string(),
                    ValType::Vec,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "b".to_string(),
                    "second vector".to_string(),
                    ValType::Vec,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "c".to_string(),
                    "third vector".to_string(),
                    ValType::Vec,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "d".to_string(),
                    "fourth vector".to_string(),
                    ValType::Vec,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "e".to_string(),
                    "fifth vector".to_string(),
                    ValType::Vec,
                    false,
                ))
                .id(),
            ],
            cmd.spawn(PortMeta::new_meta(
                "vector".to_string(),
                "a vector like: (ax * bx * cx ..., ay * by...)".to_string(),
                ValType::Vec,
                false,
            ))
            .id(),
            ass.load("nodes/vector_mul.png"),
            CyberNodes::VectorMul,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "number: multiply".to_string(),
            "multiplies an arbitrary amount of numbers together".to_string(),
            &[
                cmd.spawn(PortMeta::new_meta(
                    "a".to_string(),
                    "first number".to_string(),
                    ValType::Number,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "b".to_string(),
                    "second number".to_string(),
                    ValType::Number,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "c".to_string(),
                    "third number".to_string(),
                    ValType::Number,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "d".to_string(),
                    "fourth number".to_string(),
                    ValType::Number,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "e".to_string(),
                    "fifth number".to_string(),
                    ValType::Number,
                    false,
                ))
                .id(),
            ],
            cmd.spawn(PortMeta::new_meta(
                "sum".to_string(),
                "a * b * c * d * e".to_string(),
                ValType::Number,
                false,
            ))
            .id(),
            ass.load("nodes/multiply_number.png"),
            CyberNodes::NumberMul,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "number: subtract".to_string(),
            "subtracts an arbitrary amount of numbers in order of the input ports".to_string(),
            &[
                cmd.spawn(PortMeta::new_meta(
                    "a".to_string(),
                    "first number".to_string(),
                    ValType::Number,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "b".to_string(),
                    "second number".to_string(),
                    ValType::Number,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "c".to_string(),
                    "third number".to_string(),
                    ValType::Number,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "d".to_string(),
                    "fourth number".to_string(),
                    ValType::Number,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "e".to_string(),
                    "fifth number".to_string(),
                    ValType::Number,
                    false,
                ))
                .id(),
            ],
            cmd.spawn(PortMeta::new_meta(
                "sum".to_string(),
                "a - b - c - d - e".to_string(),
                ValType::Number,
                false,
            ))
            .id(),
            ass.load("nodes/subtract_number.png"),
            CyberNodes::NumberSub,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "store".to_string(),
            "unimplemented, sorry :(".to_string(),
            &[
                cmd.spawn(PortMeta::new_meta(
                    "0".to_string(),
                    "data slot 0".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "1".to_string(),
                    "data slot 1".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "2".to_string(),
                    "data slot 2".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "3".to_string(),
                    "data slot 3".to_string(),
                    ValType::Any,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "slot".to_string(),
                    "determines the slot to output. default is 0".to_string(),
                    ValType::Number,
                    false,
                ))
                .id(),
            ],
            cmd.spawn(PortMeta::new_meta(
                "data".to_string(),
                "the stored data from the slot `slot`".to_string(),
                ValType::Any,
                false,
            ))
            .id(),
            ass.load("nodes/storage.png"),
            CyberNodes::Storage,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "vector: create".to_string(),
            "constructs a vector from 2 numbers".to_string(),
            &[
                cmd.spawn(PortMeta::new_meta(
                    "x".to_string(),
                    "first number".to_string(),
                    ValType::Number,
                    false,
                ))
                .id(),
                cmd.spawn(PortMeta::new_meta(
                    "y".to_string(),
                    "second number".to_string(),
                    ValType::Number,
                    false,
                ))
                .id(),
            ],
            cmd.spawn(PortMeta::new_meta(
                "vector".to_string(),
                "the constructed vector".to_string(),
                ValType::Number,
                false,
            ))
            .id(),
            ass.load("nodes/vector.png"),
            CyberNodes::Vector,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "vector: negate".to_string(),
            "negates a vector".to_string(),
            &[cmd
                .spawn(PortMeta::new_meta(
                    "vector".to_string(),
                    "the vector to negate".to_string(),
                    ValType::Vec,
                    false,
                ))
                .id()],
            cmd.spawn(PortMeta::new_meta(
                "vector".to_string(),
                "the negated vector".to_string(),
                ValType::Vec,
                false,
            ))
            .id(),
            ass.load("nodes/vector_neg.png"),
            CyberNodes::VectorNeg,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "list: len".to_string(),
            "returns the length of a list".to_string(),
            &[cmd
                .spawn(PortMeta::new_meta(
                    "list".to_string(),
                    "list input".to_string(),
                    ValType::Vec,
                    false,
                ))
                .id()],
            cmd.spawn(PortMeta::new_meta(
                "length".to_string(),
                "the number of elements in the list".to_string(),
                ValType::Vec,
                false,
            ))
            .id(),
            ass.load("nodes/listlen.png"),
            CyberNodes::ListLength,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "vector: length".to_string(),
            "computes the length / magnitude of a vector".to_string(),
            &[cmd
                .spawn(PortMeta::new_meta(
                    "vector".to_string(),
                    "the vector to compute".to_string(),
                    ValType::Vec,
                    false,
                ))
                .id()],
            cmd.spawn(PortMeta::new_meta(
                "length".to_string(),
                "the length of the vector".to_string(),
                ValType::Number,
                false,
            ))
            .id(),
            ass.load("nodes/vector_length.png"),
            CyberNodes::VectorLen,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "entity: nearby".to_string(),
            "returns all nearby entities as a list of Entity".to_string(),
            &[cmd
                .spawn(PortMeta::new_meta(
                    "range".to_string(),
                    "limit to the range. default is max: 10.".to_string(),
                    ValType::Number,
                    false,
                ))
                .id()],
            cmd.spawn(PortMeta::new_meta(
                "entities".to_string(),
                "the nearby entities".to_string(),
                ValType::List,
                false,
            ))
            .id(),
            ass.load("nodes/all_entities.png"),
            CyberNodes::NearbyEntities,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "entity: direction".to_string(),
            "returns the direction the target entity is moving towards".to_string(),
            &[cmd
                .spawn(PortMeta::new_meta(
                    "target".to_string(),
                    "target entity".to_string(),
                    ValType::Entity,
                    false,
                ))
                .id()],
            cmd.spawn(PortMeta::new_meta(
                "direction".to_string(),
                "direction of the target".to_string(),
                ValType::Vec,
                false,
            ))
            .id(),
            ass.load("nodes/entity_dir.png"),
            CyberNodes::EntityDirection,
            &mut mats,
        ),
        ItemMetaBundle::new(
            "entity: position".to_string(),
            "returns the target entities current position".to_string(),
            &[cmd
                .spawn(PortMeta::new_meta(
                    "target".to_string(),
                    "target entity".to_string(),
                    ValType::Entity,
                    false,
                ))
                .id()],
            cmd.spawn(PortMeta::new_meta(
                "position".to_string(),
                "position of target entity".to_string(),
                ValType::Vec,
                false,
            ))
            .id(),
            ass.load("nodes/position.png"),
            CyberNodes::EntityPos,
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
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            ..default()
        })
        .with_children(|titlebox| {
            titlebox.spawn(TextBundle::from_section(
                "Shop",
                TextStyle {
                    font: ass.load("fonts/Geist-Regular.ttf"),
                    font_size: 62.,
                    ..default()
                },
            ));
        });

        // main area
        root.spawn(NodeBundle {
            style: Style {
                height: Val::Auto,
                width: Val::Auto,
                flex_grow: 1.,
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            ..default()
        })
        .with_children(|main| {
            main.spawn(NodeBundle {
                style: Style {
                    height: Val::Percent(100.),
                    width: Val::Auto,
                    aspect_ratio: Some(1.),
                    display: Display::Flex,
                    flex_shrink: 1.,
                    ..default()
                },
                ..default()
            })
            .with_children(|left| {
                left.spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.),
                        height: Val::Auto,
                        max_height: Val::Percent(100.),
                        flex_shrink: 0.,
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
                                    flex_shrink: 0.,
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

            main.spawn(NodeBundle {
                style: Style {
                    height: Val::Percent(100.),
                    width: Val::Auto,
                    min_width: Val::Percent(20.),
                    flex_wrap: FlexWrap::Wrap,
                    flex_basis: Val::Percent(20.),
                    max_width: Val::Auto,
                    ..default()
                },
                ..default()
            })
            .with_children(|info| {
                info.spawn((
                    TextBundle::from_section("", TextStyle::default()),
                    ItemDescription,
                ));
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

fn update_description(
    shop: Query<&ShopSelection>,
    mut desc: Query<&mut Text, With<ItemDescription>>,
    items: Query<(&Name, &Description, &PortMetas, &OutputPort), With<ItemMeta>>,
    ports: Query<&PortMeta>,
) {
    for selection in shop.iter() {
        let Some(node) = selection.node else { continue };
        let mut text = desc.single_mut();
        let (name, desc, pms, opm) = items.get(node).unwrap();

        let style = TextStyle::default();
        text.sections = vec![
            TextSection::new(&name.0, style.clone()),
            TextSection::new("\n", style.clone()),
            TextSection::new(&desc.0, style.clone()),
            TextSection::new("\n\n", style.clone()),
            TextSection::new("input ports\n", style.clone()),
        ];

        for pme in pms.0.iter() {
            let pm = ports.get(*pme).unwrap();
            text.sections
                .push(TextSection::new("name: ", style.clone()));
            text.sections
                .push(TextSection::new(&pm.name, style.clone()));
            text.sections.push(TextSection::new("\n", style.clone()));
            text.sections
                .push(TextSection::new("desc: ", style.clone()));
            text.sections
                .push(TextSection::new(&pm.desc, style.clone()));
            text.sections.push(TextSection::new("\n", style.clone()));
            text.sections
                .push(TextSection::new("type: ", style.clone()));
            text.sections
                .push(TextSection::new(pm.vt.to_string(), style.clone()));
            text.sections.push(TextSection::new("\n", style.clone()));
            text.sections.push(TextSection::new("\n", style.clone()));
        }

        text.sections
            .push(TextSection::new("output port\n", style.clone()));

        let pm = ports.get(opm.0).unwrap();
        text.sections
            .push(TextSection::new("name: ", style.clone()));
        text.sections
            .push(TextSection::new(&pm.name, style.clone()));
        text.sections.push(TextSection::new("\n", style.clone()));
        text.sections
            .push(TextSection::new("desc: ", style.clone()));
        text.sections
            .push(TextSection::new(&pm.desc, style.clone()));
        text.sections.push(TextSection::new("\n", style.clone()));
        text.sections
            .push(TextSection::new("type: ", style.clone()));
        text.sections
            .push(TextSection::new(pm.vt.to_string(), style.clone()));
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
