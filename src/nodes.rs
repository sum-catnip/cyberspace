use std::{collections::HashMap, fmt, marker::PhantomData};

use bevy::prelude::*;
use hexx::{EdgeDirection, Hex};

use crate::{Map, TileType};

#[derive(Component, Clone)]
pub struct Name(pub String);

#[derive(Component, Clone)]
pub struct Description(pub String);

#[derive(Component, Clone)]
pub struct ItemMeta;

#[derive(Component, Clone)]
pub struct PortMeta {
    pub name: String,
    pub desc: String,
    pub vt: ValType,
    pub constant: bool,
}

// links to metaport
#[derive(Component, Default)]
pub struct PortCfg {
    pub inputs: HashMap<EdgeDirection, Entity>,
    pub constant: Option<Val>,
}

#[derive(Component, Clone)]
pub struct PortMetas(pub Vec<Entity>);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ValType {
    Entity,
    Vec,
    Number,
    Text,
    List,
}

#[derive(Clone, PartialEq)]
pub enum Val {
    Empty,
    Entity(Entity),
    Vec(Vec2),
    Number(f64),
    Text(String),
    List(Vec<Val>),
}

#[derive(Component)]
pub struct TargetableEntity;

#[derive(Component)]
pub struct Health(pub f64);

#[derive(Component)]
pub struct MetaLink(pub Entity);

#[derive(Component, Default, PartialEq)]
pub enum CyberState {
    #[default]
    Idle,
    ActivationRequest,
    Done(Result<Val, ()>),
}

#[derive(Component)]
pub struct HexPos(pub Hex);

#[derive(Bundle)]
pub struct NodeBundle {
    pub meta: MetaLink,
    pub cfg: PortCfg,
    pub state: CyberState,
    pub pos: HexPos,
}

#[derive(Component, Clone, Copy)]
pub struct OutputPort(Option<Entity>);

#[derive(Bundle, Clone)]
pub struct ItemMetaBundle {
    pub name: Name,
    pub desc: Description,
    pub tex: Handle<Image>,
    pub mat: Handle<ColorMaterial>,
    pub ports: PortMetas,
    pub output: OutputPort,
    pub node: CyberNodes,
    meta: ItemMeta,
}

impl fmt::Display for ValType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl PortMeta {
    pub fn new_meta(name: String, desc: String, t: ValType, constant: bool) -> Self {
        Self {
            name,
            desc,
            vt: t,
            constant,
        }
    }
}

impl ItemMetaBundle {
    pub fn new(
        name: String,
        desc: String,
        ports: &[Entity],
        outputs: Option<Entity>,
        tex: Handle<Image>,
        mat: Handle<ColorMaterial>,
        node: CyberNodes,
    ) -> Self {
        Self {
            name: Name(name),
            desc: Description(desc),
            mat,
            tex,
            ports: PortMetas(ports.to_vec()),
            output: OutputPort(outputs),
            meta: ItemMeta,
            node,
        }
    }
}

#[derive(Component, Clone, Copy)]
pub enum CyberNodes {
    WIP,
    Lazor,
    ClosestEntity,
}

#[derive(Event)]
pub struct TickNode<T> {
    pub e: Entity,
    _pd: PhantomData<T>,
}

impl<T> TickNode<T> {
    pub fn new(e: Entity) -> Self {
        Self {
            e,
            _pd: PhantomData::default(),
        }
    }
}

#[derive(Default)]
pub struct Lazor;
#[derive(Default)]
pub struct ClosestEntity;

pub struct CyberPlugin;
impl Plugin for CyberPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TickNode<Lazor>>()
            .add_event::<TickNode<ClosestEntity>>()
            .add_systems(Update, (lazor_tick, closest_tick));
    }
}

fn port_by_name(
    name: &str,
    pos: Hex,
    map: &Map,
    cfg: &PortCfg,
    meta: &Query<&PortMeta>,
    tiles: &Query<&TileType>,
) -> Entity {
    *cfg.inputs
        .iter()
        .find_map(|(h, e)| (meta.get(*e).unwrap().name == name).then_some(h))
        .map(|h| match tiles.get(map.fetch_panic(pos + *h)).unwrap() {
            TileType::CyberNode { e, .. } => e,
            _ => unreachable!(),
        })
        .unwrap()
}

fn lazor_tick(
    mut evt: EventReader<TickNode<Lazor>>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    tiles: Query<&TileType>,
    mut state: Query<&mut CyberState>,
    metas: Query<&PortMeta>,
    mut targets: Query<&mut Health, With<TargetableEntity>>,
) {
    const DMG: f64 = 10.;
    for e in evt.read() {
        info!("ticking lazor");
        let Ok((pos, cfg)) = node.get(e.e) else {
            return;
        };

        let target = port_by_name("target", pos.0, &map, cfg, &metas, &tiles);
        let Ok(target_state) = state.get(target) else {
            warn!("port tile no longer exists");
            continue;
        };

        let CyberState::Done(Ok(Val::Entity(target))) = *target_state else {
            warn!("port tile changed state unexpectedly, or has invalid type");
            continue;
        };

        let mut state = state.get_mut(e.e).unwrap();
        let Ok(mut hp) = targets.get_mut(target) else {
            // entity no longer exists
            warn!("tried to lazor entity that no longer exists or has no health");
            *state = CyberState::Done(Err(()));
            continue;
        };

        hp.0 -= DMG;
        *state = CyberState::Idle;
    }
}

fn closest_tick(
    map: Res<Map>,
    mut evt: EventReader<TickNode<ClosestEntity>>,
    mut state: Query<(&mut CyberState, &HexPos)>,
    targets: Query<(Entity, &Transform), With<TargetableEntity>>,
) {
    for tick in evt.read() {
        info!("ticking get closest entity");
        let (mut state, hpos) = state.get_mut(tick.e).unwrap();
        let e = targets.iter().find_map(|(e, trans)| {
            let pos = map.layout.hex_to_world_pos(hpos.0);
            let dist = (trans.translation.xy() - pos).length();
            (dist <= 25.).then_some(e)
        });

        *state = match e {
            Some(e) => CyberState::Done(Ok(Val::Entity(e))),
            None => {
                error!("closest entity: no entities in range");
                CyberState::Done(Err(()))
            }
        };
    }
}
