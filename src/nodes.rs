use std::{collections::HashMap, fmt, marker::PhantomData};

use bevy::prelude::*;
use hexx::{EdgeDirection, Hex};

use crate::Map;

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
}

// links to metaport
#[derive(Component, Default)]
pub struct PortCfg(pub HashMap<EdgeDirection, Entity>);

#[derive(Component, Clone)]
pub struct PortMetas(pub Vec<Entity>);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ValType {
    Entity,
    Vec,
    Number,
    Text,
}

#[derive(Clone, PartialEq)]
pub enum Val {
    Entity(Entity),
    Vec(Vec2),
    Number(f64),
    Text(String),
}

#[derive(Component)]
pub struct TargetableEntity;

#[derive(Component)]
pub struct Health(f64);

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
    pub outputs: OutputPort,
    pub node: CyberNodes,
    meta: ItemMeta,
}

impl fmt::Display for ValType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl PortMeta {
    pub fn new_meta(name: String, desc: String, t: ValType) -> Self {
        Self { name, desc, vt: t }
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
            outputs: OutputPort(outputs),
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

fn lazor_tick(mut evt: EventReader<TickNode<Lazor>>) {
    for _ in evt.read() {
        info!("ticking lazor");
    }
}

fn closest_tick(
    map: Res<Map>,
    mut evt: EventReader<TickNode<ClosestEntity>>,
    mut state: Query<(&mut CyberState, &HexPos)>,
    targets: Query<(Entity, &Transform, &TargetableEntity)>,
) {
    for tick in evt.read() {
        info!("ticking get closest entity");
        let (mut state, hpos) = state.get_mut(tick.e).unwrap();
        let e = targets.iter().find_map(|(e, trans, _)| {
            let pos = map.layout.hex_to_world_pos(hpos.0);
            let dist = (trans.translation.xy() - pos).length();
            (dist > 100.).then_some(e)
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
