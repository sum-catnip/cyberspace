use std::{collections::HashMap, f32::consts::PI, fmt, marker::PhantomData};

use bevy::{
    ecs::system::QueryLens,
    gizmos::gizmos,
    math::{
        bounding::{BoundingCircle, IntersectsVolume},
        quat, vec2, vec3, NormedVectorSpace, VectorSpace,
    },
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
use hexx::{EdgeDirection, Hex};

use crate::{enemy::PathfindPath, CommonResources, Gamestate, Map, TileType};

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
    pub inputs: HashMap<Hex, Entity>,
    pub constant: Option<Val>,
}

#[derive(Component, Clone)]
pub struct PortMetas(pub Vec<Entity>);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ValType {
    Empty,
    Any,
    Entity,
    Vec,
    Number,
    Text,
    List,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Empty,
    Entity(Entity),
    Vec(Vec2),
    Number(f32),
    Text(String),
    List(Vec<Val>),
}

#[derive(Component)]
pub struct TargetableEntity;

#[derive(Component)]
pub struct Health(pub f32);

#[derive(Component)]
pub struct MetaLink(pub Entity);

#[derive(Component, Default, PartialEq)]
pub enum CyberState {
    #[default]
    Idle,
    ActivationRequest,
    Triggered,
    Done(Result<Val, ()>),
    Disabled,
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
pub struct OutputPort(pub Entity);

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
        outputs: Entity,
        tex: Handle<Image>,
        node: CyberNodes,
        mats: &mut Assets<ColorMaterial>,
    ) -> Self {
        Self {
            name: Name(name),
            desc: Description(desc),
            mat: mats.add(ColorMaterial {
                texture: Some(tex.clone()),
                color: Color::srgb(2.1, 2.1, 2.1),
            }),
            tex,
            ports: PortMetas(ports.to_vec()),
            output: OutputPort(outputs),
            meta: ItemMeta,
            node,
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub enum CyberNodes {
    WIP,
    Lazor,
    List,
    Debug,
    Project,
    Shock,
    Plasma,
    Orbital,
    RocketLauncher,
    ClosestEntity,
    EntityDirection,
    NearbyEntities,
    EntityPos,
    ConstantNumber,
    NumberSub,
    Storage,
    NumberMul,
    VectorMul,
    VectorNeg,
    ListLength,
    VectorLen,
    Vector,
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
pub struct Project;
#[derive(Default)]
pub struct Plasma;
#[derive(Default)]
pub struct Shock;
#[derive(Default)]
pub struct Orbital;
#[derive(Default)]
pub struct RocketLauncher;
#[derive(Default)]
pub struct Debug;
#[derive(Default)]
pub struct Storage;
#[derive(Default)]
pub struct ClosestEntity;
#[derive(Default)]
pub struct EntityPos;
#[derive(Default)]
pub struct EntityDirection;
#[derive(Default)]
pub struct NearbyEntity;
#[derive(Default)]
pub struct ConstantNumber;
#[derive(Default)]
pub struct List;
#[derive(Default)]
pub struct ListLen;
#[derive(Default)]
pub struct VectorMul;
#[derive(Default)]
pub struct VectorLen;
#[derive(Default)]
pub struct Vector;
#[derive(Default)]
pub struct VectorNeg;
#[derive(Default)]
pub struct NumberMul;
#[derive(Default)]
pub struct NumberSub;

pub struct CyberPlugin;
impl Plugin for CyberPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TickNode<Lazor>>()
            .add_event::<TickNode<RocketLauncher>>()
            .add_event::<TickNode<Orbital>>()
            .add_event::<TickNode<Shock>>()
            .add_event::<TickNode<Project>>()
            .add_event::<TickNode<Plasma>>()
            .add_event::<TickNode<Debug>>()
            .add_event::<TickNode<ListLen>>()
            .add_event::<TickNode<ClosestEntity>>()
            .add_event::<TickNode<NearbyEntity>>()
            .add_event::<TickNode<EntityDirection>>()
            .add_event::<TickNode<EntityPos>>()
            .add_event::<TickNode<ConstantNumber>>()
            .add_event::<TickNode<List>>()
            .add_event::<TickNode<VectorMul>>()
            .add_event::<TickNode<VectorLen>>()
            .add_event::<TickNode<VectorNeg>>()
            .add_event::<TickNode<Vector>>()
            .add_event::<TickNode<NumberMul>>()
            .add_event::<TickNode<NumberSub>>()
            .add_systems(
                Update,
                (
                    rocket_anim,
                    lazorbeam_anim,
                    plasma_anim,
                    explosion_anim,
                    orbital_target,
                    tesla_anim,
                )
                    .run_if(in_state(Gamestate::Game)),
            )
            .add_systems(
                FixedUpdate,
                (
                    lazor_tick,
                    closest_tick,
                    constant_tick,
                    list_tick,
                    listlen_tick,
                    entity_pos_tick,
                    entity_dir_tick,
                    rocket_launcher_tick,
                    orbital_tick,
                    tesla_tick,
                    project_tick,
                    plasma_tick,
                    vecmul_tick,
                    nummul_tick,
                    numsub_tick,
                    debug_tick,
                    vec_tick,
                    veclen_tick,
                    vecneg_tick,
                    nearby_tick,
                )
                    .run_if(in_state(Gamestate::Game)),
            );
    }
}

type TickEvts<'a, 'b, T> = EventReader<'a, 'b, TickNode<T>>;

fn port_by_name(
    name: &str,
    pos: Hex,
    map: &Map,
    cfg: &PortCfg,
    meta: &mut QueryLens<&PortMeta>,
    tiles: &mut QueryLens<&TileType>,
) -> Option<Entity> {
    let meta = meta.query();
    let tiles = tiles.query();
    cfg.inputs
        .iter()
        .find_map(|(h, e)| (meta.get(*e).unwrap().name == name).then_some(h))
        .map(|h| match tiles.get(map.fetch_panic(pos + *h)).unwrap() {
            TileType::CyberNode { e, .. } => e,
            _ => unreachable!(),
        })
        .copied()
}

fn fetch_port_data(
    name: &str,
    pos: Hex,
    map: &Map,
    cfg: &PortCfg,
    metas: &mut QueryLens<&PortMeta>,
    states: &mut QueryLens<&CyberState>,
    tiles: &mut QueryLens<&TileType>,
) -> Result<Val, ()> {
    let states = states.query();

    let Some(target) = port_by_name(name, pos, &map, cfg, metas, tiles) else {
        error!("{:?} port not configured", name);
        return Err(());
    };

    let Ok(target_state) = states.get(target) else {
        warn!("port {:?} tile no longer exists", name);
        return Err(());
    };

    let CyberState::Done(Ok(v)) = target_state else {
        warn!(
            "port {:?} tile changed state unexpectedly, or has invalid type",
            name
        );
        return Err(());
    };

    return Ok(v.clone());
}

#[derive(Component)]
struct Target(Entity);

#[derive(Component)]
struct Lazorbeam;

#[derive(Bundle)]
struct LazorbeamBundle {
    apperance: MaterialMesh2dBundle<ColorMaterial>,
    target: Target,
    marker: Lazorbeam,
}

fn lazor_tick(
    mut evt: TickEvts<Lazor>,
    mut cmd: Commands,
    res: Res<CommonResources>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut state: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
    mut targets: Query<&mut Health, With<TargetableEntity>>,
) {
    const DMG: f32 = 15.;
    for e in evt.read() {
        info!("ticking lazor");
        let Ok((pos, cfg)) = node.get(e.e) else {
            return;
        };

        let Some(target) = port_by_name(
            "target",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut tiles.as_query_lens(),
        ) else {
            error!("lazor: 'target' port not configured");
            let mut state = state.get_mut(e.e).unwrap();
            *state = CyberState::Done(Err(()));
            return;
        };
        let Ok(target_state) = state.get(target) else {
            warn!("lazor: port tile no longer exists");
            continue;
        };

        let CyberState::Done(Ok(Val::Entity(target))) = *target_state else {
            warn!("lazor: port tile changed state unexpectedly, or has invalid type");
            continue;
        };

        let mut state = state.get_mut(e.e).unwrap();
        let Ok(mut hp) = targets.get_mut(target) else {
            // entity no longer exists
            warn!("lazor: tried to lazor entity that no longer exists or has no health");
            *state = CyberState::Done(Err(()));
            continue;
        };

        cmd.spawn(LazorbeamBundle {
            target: Target(target),
            apperance: MaterialMesh2dBundle {
                visibility: Visibility::Hidden,
                transform: Transform::from_translation(
                    map.layout.hex_to_world_pos(pos.0).extend(2.),
                ),
                mesh: res.lazer.clone(),
                material: res.lazer_mat.clone(),
                ..default()
            },
            marker: Lazorbeam,
        });

        hp.0 -= DMG;
        *state = CyberState::Done(Ok(Val::Empty));
    }
}

fn lazorbeam_anim(
    mut cmd: Commands,
    time: Res<Time>,
    mut beams: Query<
        (Entity, &mut Transform, &Target, &mut Visibility),
        (With<Lazorbeam>, Without<TargetableEntity>),
    >,
    targets: Query<&Transform, With<TargetableEntity>>,
) {
    for (ent, mut trans, target, mut vis) in beams.iter_mut() {
        let Ok(pos) = targets.get(target.0) else {
            // target no longer exists
            cmd.entity(ent).despawn();
            continue;
        };

        let dir = Dir2::new((pos.translation - trans.translation).xy()).unwrap();
        trans.translation += (dir * 3000. * time.delta_seconds()).extend(0.);
        let to_enemy = (pos.translation.xy() - trans.translation.xy()).normalize();
        let rotate_to_enemy = Quat::from_rotation_arc(Vec3::Y, to_enemy.extend(0.));
        trans.rotation = rotate_to_enemy;

        if trans.translation.distance(pos.translation) <= 50. {
            cmd.entity(ent).despawn();
        }

        *vis = Visibility::Visible;
    }
}

fn rocket_anim(
    mut cmd: Commands,
    time: Res<Time>,
    mut rockets: Query<
        (Entity, &Direction, &mut Transform, &mut Visibility),
        (With<Rocket>, Without<TargetableEntity>),
    >,
    mut targets: Query<(&Transform, &mut Health), With<TargetableEntity>>,
) {
    for (ent, dir, mut trans, mut vis) in rockets.iter_mut() {
        let bb = BoundingCircle::new(trans.translation.xy(), 10.);
        let mut hit = false;
        for (tt, mut hp) in targets.iter_mut() {
            if bb.intersects(&BoundingCircle::new(tt.translation.xy(), 7.)) {
                hit = true;
                hp.0 -= 50.;
            }
        }

        if hit || trans.translation.xy().distance(Vec2::ZERO) >= 1000. {
            cmd.entity(ent).despawn();
        }

        trans.translation += (dir.0 * 300. * time.delta_seconds()).extend(0.);
        let rotate_to_enemy = Quat::from_rotation_arc(Vec3::Y, dir.0.extend(0.));
        trans.rotation = rotate_to_enemy;
        *vis = Visibility::Visible;
    }
}

#[derive(Component)]
struct Direction(Vec2);

#[derive(Component)]
struct Rocket;

#[derive(Bundle)]
struct RocketBundle {
    dir: Direction,
    apperance: MaterialMesh2dBundle<ColorMaterial>,
    marker: Rocket,
}

#[derive(Component)]
struct OrbitalTimer(Timer);

#[derive(Component)]
struct OrbitalMarker;

#[derive(Bundle)]
struct OrbitalBundle {
    timer: OrbitalTimer,
    target: HexPos,
    apperance: SpriteBundle,
    marker: OrbitalMarker,
}

fn rocket_launcher_tick(
    mut cmd: Commands,
    mut evt: TickEvts<RocketLauncher>,
    res: Res<CommonResources>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut state: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
) {
    for e in evt.read() {
        info!("ticking rocket launcher");
        let Ok((pos, cfg)) = node.get(e.e) else {
            return;
        };

        let Some(dir) = port_by_name(
            "direction",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut tiles.as_query_lens(),
        ) else {
            error!("launcher: 'direction' port not configured");
            let mut state = state.get_mut(e.e).unwrap();
            *state = CyberState::Done(Err(()));
            return;
        };
        let Ok(target_state) = state.get(dir) else {
            warn!("launcher: port tile no longer exists");
            continue;
        };

        let CyberState::Done(Ok(Val::Vec(dir))) = *target_state else {
            warn!("launcher: 'direction' port error or has invalid type");
            continue;
        };

        cmd.spawn(RocketBundle {
            dir: Direction(dir.normalize()),
            apperance: MaterialMesh2dBundle {
                transform: Transform::from_translation(
                    map.layout.hex_to_world_pos(pos.0).extend(2.),
                ),
                mesh: res.missile.clone(),
                material: res.missile_mat.clone(),
                ..default()
            },
            marker: Rocket,
        });
        let mut state = state.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Empty));
    }
}

fn orbital_tick(
    mut cmd: Commands,
    mut evt: TickEvts<Orbital>,
    common: Res<CommonResources>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut states: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
) {
    for e in evt.read() {
        info!("ticking orbital strike");
        let Ok((pos, cfg)) = node.get(e.e) else {
            continue;
        };

        let Ok(Val::Vec(pos)) = fetch_port_data(
            "target",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut states.transmute_lens(),
            &mut tiles.as_query_lens(),
        ) else {
            continue;
        };

        cmd.spawn(OrbitalBundle {
            marker: OrbitalMarker,
            timer: OrbitalTimer(Timer::from_seconds(10., TimerMode::Once)),
            target: HexPos(Hex::round(pos.to_array())),
            apperance: SpriteBundle {
                transform: Transform::from_translation(
                    map.layout.fract_hex_to_world_pos(pos).extend(2.),
                )
                .with_scale(Vec3::splat(0.3)),
                texture: common.target_img.clone(),
                ..default()
            },
        });

        let mut state = states.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Empty));
    }
}

#[derive(Component)]
struct Explosion;

#[derive(Bundle)]
struct ExplosionBundle {
    apperance: MaterialMesh2dBundle<ColorMaterial>,
    timer: OrbitalTimer,
    marker: Explosion,
}

fn orbital_target(
    mut cmd: Commands,
    time: Res<Time>,
    common: Res<CommonResources>,
    mut mats: ResMut<Assets<ColorMaterial>>,
    mut timers: Query<(Entity, &Transform, &mut OrbitalTimer)>,
    mut targets: Query<(&Transform, &mut Health), With<TargetableEntity>>,
) {
    for (e, trans, mut timer) in timers.iter_mut() {
        if !timer.0.tick(time.delta()).just_finished() {
            continue;
        }

        let bb = BoundingCircle::new(trans.translation.xy(), 200.);
        for (tt, mut hp) in targets.iter_mut() {
            if bb.intersects(&BoundingCircle::new(tt.translation.xy(), 5.)) {
                hp.0 -= 5.;
            }
        }

        cmd.entity(e).despawn();
        cmd.spawn(ExplosionBundle {
            apperance: MaterialMesh2dBundle {
                transform: trans.clone().with_scale(Vec3::splat(0.)),
                mesh: common.explosion.clone(),
                material: mats.add(Color::srgb(5., 3., 0.6)).into(),
                ..default()
            },
            marker: Explosion,
            timer: OrbitalTimer(Timer::from_seconds(0.5, TimerMode::Once)),
        });
    }
}

fn explosion_anim(
    mut cmd: Commands,
    time: Res<Time>,
    mut mats: ResMut<Assets<ColorMaterial>>,
    mut explosions: Query<
        (
            Entity,
            &mut OrbitalTimer,
            &mut Transform,
            &Handle<ColorMaterial>,
        ),
        With<Explosion>,
    >,
) {
    for (e, mut timer, mut trans, mat) in explosions.iter_mut() {
        if timer.0.tick(time.delta()).finished() {
            cmd.entity(e).despawn();
        }

        trans.scale = Vec3::splat(timer.0.fraction());
        let color = vec3(5., 3., 0.6) * timer.0.remaining_secs();
        let color = Color::srgb(color.x, color.y, color.z).with_alpha(timer.0.fraction_remaining());
        mats.get_mut(mat.id()).unwrap().color = color;
    }
}

#[derive(Component)]
struct PlasmaCounter(u32);

#[derive(Component)]
struct PlasmaMarker;

#[derive(Component)]
struct PlasmaTimer(Timer);

#[derive(Component)]
struct DmgTimer(Timer);

#[derive(Bundle)]
struct PlasmaBundle {
    target: HexPos,
    time: PlasmaTimer,
    dmg_timer: DmgTimer,
    apperance: MaterialMesh2dBundle<ColorMaterial>,
    marker: PlasmaMarker,
}

fn plasma_tick(
    mut cmd: Commands,
    mut evt: TickEvts<Plasma>,
    common: Res<CommonResources>,
    mut mats: ResMut<Assets<ColorMaterial>>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut states: Query<&mut CyberState>,
    mut counters: Query<&mut PlasmaCounter>,
    mut metas: Query<&PortMeta>,
) {
    for e in evt.read() {
        info!("ticking plasma cannon");
        let Ok((pos, cfg)) = node.get(e.e) else {
            continue;
        };

        let Ok(Val::Vec(target)) = fetch_port_data(
            "target",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut states.transmute_lens(),
            &mut tiles.as_query_lens(),
        ) else {
            continue;
        };

        let threshold = match fetch_port_data(
            "threshold",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut states.transmute_lens(),
            &mut tiles.as_query_lens(),
        ) {
            Ok(Val::Number(t)) => t,
            _ => 0.,
        };

        let mut count = counters.get(e.e).map(|c| c.0).unwrap_or(0) + 1;
        let mut res = 0;
        if count as f32 >= threshold {
            let color = vec3(0.541, 0.168, 0.886) * count as f32;
            cmd.spawn(PlasmaBundle {
                target: HexPos(Hex::round(target.to_array())),
                time: PlasmaTimer(Timer::from_seconds(count as f32 / 2., TimerMode::Once)),
                marker: PlasmaMarker,
                dmg_timer: DmgTimer(Timer::from_seconds(0.5, TimerMode::Repeating)),
                apperance: MaterialMesh2dBundle {
                    transform: Transform::from_translation(
                        map.layout.fract_hex_to_world_pos(target).extend(2.),
                    ),
                    //transform: Transform::from_translation(Vec2::ZERO.extend(2.)),
                    mesh: common.plasma.clone(),
                    material: mats.add(Color::srgb(color.x, color.y, color.z)),
                    ..default()
                },
            });

            res = count;
            count = 0;
        }

        if let Ok(mut counter) = counters.get_mut(e.e) {
            counter.0 = count;
        } else {
            cmd.entity(e.e).insert(PlasmaCounter(count));
        }

        let mut state = states.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Number(res as f32)));
    }
}

fn plasma_anim(
    mut cmd: Commands,
    time: Res<Time>,
    mut mats: ResMut<Assets<ColorMaterial>>,
    mut plasmas: Query<
        (
            Entity,
            &mut PlasmaTimer,
            &mut DmgTimer,
            &Transform,
            &Handle<ColorMaterial>,
        ),
        (With<PlasmaMarker>, Without<TargetableEntity>),
    >,
    mut targets: Query<(&Transform, &mut Health), With<TargetableEntity>>,
) {
    for (e, mut timer, mut dmgtimer, trans, mat) in plasmas.iter_mut() {
        if timer.0.tick(time.delta()).finished() {
            cmd.entity(e).despawn();
        }

        let color = vec3(0.541, 0.168, 0.886) * timer.0.remaining_secs();
        let color = Color::srgb(color.x, color.y, color.z).with_alpha(timer.0.fraction_remaining());
        mats.get_mut(mat.id()).unwrap().color = color;

        let dmg = timer.0.remaining_secs() * 5.;
        if dmgtimer.0.tick(time.delta()).just_finished() {
            let bb = BoundingCircle::new(trans.translation.xy(), 100.);
            for (tt, mut hp) in targets.iter_mut() {
                if bb.intersects(&BoundingCircle::new(tt.translation.xy(), 5.)) {
                    hp.0 -= dmg;
                }
            }
        }
    }
}

#[derive(Component)]
struct TeslaTimer(Timer);

#[derive(Component)]
struct TeslaTargets(Vec<Entity>);

#[derive(Component)]
struct TeslaOrig(Vec2);

fn tesla_tick(
    mut evt: TickEvts<Shock>,
    mut cmd: Commands,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut states: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
    mut targetable: Query<&mut Health, With<TargetableEntity>>,
) {
    const DMG: f32 = 5.;
    for e in evt.read() {
        info!("ticking tesla coil");
        let Ok((pos, cfg)) = node.get(e.e) else {
            continue;
        };

        let Ok(Val::List(targets)) = fetch_port_data(
            "targets",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut states.transmute_lens(),
            &mut tiles.as_query_lens(),
        ) else {
            continue;
        };

        let mut res = 0;
        let mut ents = Vec::new();
        for t in targets {
            let Val::Entity(e) = t else {
                warn!("tesla coil skipping input because its not of type entity");
                continue;
            };

            let Ok(mut hp) = targetable.get_mut(e) else {
                warn!("tesla coil: target no longer exists");
                continue;
            };

            ents.push(e);
            hp.0 -= DMG;
            res += 1;
        }
        cmd.spawn((
            TeslaTimer(Timer::from_seconds(0.3, TimerMode::Once)),
            TeslaTargets(ents),
            TeslaOrig(map.layout.hex_to_world_pos(pos.0)),
        ));

        let mut state = states.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Number(res as f32)));
    }
}

fn tesla_anim(
    mut cmd: Commands,
    time: Res<Time>,
    mut giz: Gizmos,
    mut q: Query<(Entity, &mut TeslaTimer, &TeslaTargets, &TeslaOrig)>,
    transforms: Query<&Transform, With<TargetableEntity>>,
) {
    for (e, mut timer, targets, orig) in q.iter_mut() {
        if timer.0.tick(time.delta()).finished() {
            cmd.entity(e).despawn()
        };

        for te in targets.0.iter() {
            let Ok(tt) = transforms.get(*te) else {
                continue;
            };
            giz.line_2d(orig.0, tt.translation.xy(), Color::srgb(0., 0.1, 5.));
        }
    }
}

fn project_tick(
    mut evt: TickEvts<Project>,
    mut cmd: Commands,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&mut TileType>,
    mut states: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
    mut mats: Query<&mut Handle<ColorMaterial>>,
    common: Res<CommonResources>,
) {
    for e in evt.read() {
        info!("ticking project tile");
        let Ok((pos, cfg)) = node.get(e.e) else {
            continue;
        };

        let Ok(Val::Vec(pos)) = fetch_port_data(
            "target",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut states.transmute_lens(),
            &mut tiles.transmute_lens(),
        ) else {
            continue;
        };

        let pos = Hex::round(pos.to_array());
        let Some(tilee) = map.storage.get(pos) else {
            error!("project tile: out of bounds");
            let mut state = states.get_mut(e.e).unwrap();
            *state = CyberState::Done(Err(()));
            continue;
        };

        let mut tt = tiles.get_mut(*tilee).unwrap();
        if *tt != TileType::Unoccupied {
            error!("project tile: tile already occupied");
            let mut state = states.get_mut(e.e).unwrap();
            *state = CyberState::Done(Err(()));
            continue;
        }

        let ent = cmd.spawn((Health(10.), HexPos(pos))).id();
        *tt = TileType::Terrain(ent);
        *mats.get_mut(*tilee).unwrap() = common.illusion.clone();

        let mut state = states.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Empty));
    }
}

fn nearby_tick(
    map: Res<Map>,
    mut evt: TickEvts<NearbyEntity>,
    mut states: Query<(&mut CyberState, &HexPos)>,
    targets: Query<(Entity, &Transform), With<TargetableEntity>>,
) {
    for e in evt.read() {
        info!("ticking nearby entities");
        let (mut state, hpos) = states.get_mut(e.e).unwrap();

        let targets = targets.iter().filter_map(|(e, trans)| {
            let pos = map.layout.world_pos_to_hex(trans.translation.xy());
            (pos.distance_to(hpos.0) <= 8).then_some(e)
        });

        *state = CyberState::Done(Ok(Val::List(targets.map(|e| Val::Entity(e)).collect())));
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
            let pos = map.layout.world_pos_to_hex(trans.translation.xy());
            (pos.distance_to(hpos.0) <= 8).then_some(e)
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

fn constant_tick(mut evt: TickEvts<ConstantNumber>, mut state: Query<(&mut CyberState, &PortCfg)>) {
    for tick in evt.read() {
        info!("ticking constant number");
        let Ok((mut state, cfg)) = state.get_mut(tick.e) else {
            // node removed
            continue;
        };

        let Some(constant) = cfg.constant.as_ref() else {
            error!("constant number: no constant configured");
            *state = CyberState::Done(Err(()));
            continue;
        };

        *state = CyberState::Done(Ok(constant.clone()));
    }
}

fn listlen_tick(
    mut evt: TickEvts<ListLen>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut states: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
) {
    for e in evt.read() {
        info!("ticking list length");
        let Ok((pos, cfg)) = node.get(e.e) else {
            return;
        };

        let Ok(Val::List(list)) = fetch_port_data(
            "list",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut states.transmute_lens(),
            &mut tiles.as_query_lens(),
        ) else {
            error!("list len: no input list");
            let mut state = states.get_mut(e.e).unwrap();
            *state = CyberState::Done(Err(()));
            return;
        };

        let mut state = states.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Number(list.len() as f32)));
    }
}

fn list_tick(
    mut evt: EventReader<TickNode<List>>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut state: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
) {
    for e in evt.read() {
        info!("ticking construct list");
        let Ok((pos, cfg)) = node.get(e.e) else {
            continue;
        };

        let ports = ["a", "b", "c", "d", "e"].iter().filter_map(|name| {
            port_by_name(
                name,
                pos.0,
                &map,
                cfg,
                &mut metas.as_query_lens(),
                &mut tiles.as_query_lens(),
            )
        });

        let mut list = Vec::new();
        for p in ports {
            let Ok(target_state) = state.get(p) else {
                warn!("list: port tile no longer exists");
                continue;
            };

            let CyberState::Done(Ok(val)) = target_state else {
                warn!("list: input tile errored");
                continue;
            };

            match val {
                Val::Empty => continue,
                Val::Entity(e) => list.push(Val::Entity(*e)),
                Val::Vec(v) => list.push(Val::Vec(*v)),
                Val::Number(n) => list.push(Val::Number(*n)),
                Val::Text(t) => list.push(Val::Text(t.to_string())),
                Val::List(l) => {
                    for i in l {
                        list.push(i.clone());
                    }
                }
            }
        }

        let mut state = state.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::List(list)));
    }
}

fn entity_dir_tick(
    mut evt: TickEvts<EntityDirection>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut states: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
    targets: Query<(&Transform, &PathfindPath), With<TargetableEntity>>,
) {
    for e in evt.read() {
        info!("ticking entity direction");
        let Ok((pos, cfg)) = node.get(e.e) else {
            return;
        };

        let Ok(Val::Entity(te)) = fetch_port_data(
            "target",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut states.transmute_lens(),
            &mut tiles.as_query_lens(),
        ) else {
            error!("entity direction: 'target' port not configured");
            let mut state = states.get_mut(e.e).unwrap();
            *state = CyberState::Done(Err(()));
            return;
        };

        let Ok((pos, path)) = targets.get(te) else {
            warn!("entity dir: target no longer exists");
            continue;
        };

        let to = map.layout.hex_to_world_pos(path.path[path.i]);
        let dir = (to - pos.translation.xy()).normalize_or_zero();

        let mut state = states.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Vec(dir)));
    }
}

fn entity_pos_tick(
    mut evt: TickEvts<EntityPos>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut state: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
    targets: Query<&Transform, With<TargetableEntity>>,
) {
    for e in evt.read() {
        info!("ticking entity position");
        let Ok((pos, cfg)) = node.get(e.e) else {
            return;
        };

        let Some(target) = port_by_name(
            "target",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut tiles.as_query_lens(),
        ) else {
            error!("entity position: 'target' port not configured");
            let mut state = state.get_mut(e.e).unwrap();
            *state = CyberState::Done(Err(()));
            return;
        };

        let Ok(target_state) = state.get(target) else {
            warn!("entity position: port tile doesnt exist");
            continue;
        };

        let CyberState::Done(Ok(Val::Entity(target))) = *target_state else {
            warn!("entity position: 'target' port tile with error");
            continue;
        };

        let mut state = state.get_mut(e.e).unwrap();
        let Ok(transform) = targets.get(target) else {
            // entity no longer exists
            warn!("entity position: target no longer exists");
            *state = CyberState::Done(Err(()));
            continue;
        };

        *state = CyberState::Done(Ok(Val::Vec(
            map.layout
                .world_pos_to_fract_hex(transform.translation.xy()),
        )));
    }
}

fn debug_tick(
    mut evt: TickEvts<Debug>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut states: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
) {
    for e in evt.read() {
        info!("ticking debug");
        let Ok((pos, cfg)) = node.get(e.e) else {
            return;
        };
        let ports = ["a", "b", "c", "d", "e"];
        let inputs = ports.iter().flat_map(|p| {
            fetch_port_data(
                p,
                pos.0,
                &map,
                cfg,
                &mut metas.as_query_lens(),
                &mut states.transmute_lens(),
                &mut tiles.as_query_lens(),
            )
            .ok()
        });

        for input in inputs {
            info!("debug input: {:?}", input);
        }

        let mut state = states.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Empty));
    }
}

fn vecneg_tick(
    mut evt: TickEvts<VectorNeg>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut states: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
) {
    for e in evt.read() {
        info!("ticking vector negate");
        let Ok((pos, cfg)) = node.get(e.e) else {
            return;
        };

        let Ok(Val::Vec(x)) = fetch_port_data(
            "vector",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut states.transmute_lens(),
            &mut tiles.as_query_lens(),
        ) else {
            error!("vector len: no or invalid x port");
            continue;
        };

        let mut state = states.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Vec(-x)));
    }
}

fn veclen_tick(
    mut evt: TickEvts<VectorLen>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut states: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
) {
    for e in evt.read() {
        info!("ticking vector len");
        let Ok((pos, cfg)) = node.get(e.e) else {
            return;
        };

        let Ok(Val::Vec(x)) = fetch_port_data(
            "vector",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut states.transmute_lens(),
            &mut tiles.as_query_lens(),
        ) else {
            error!("vector len: no or invalid x port");
            continue;
        };

        let mut state = states.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Number(x.length())));
    }
}

fn vec_tick(
    mut evt: TickEvts<Vector>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut states: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
) {
    for e in evt.read() {
        info!("ticking vector create");
        let Ok((pos, cfg)) = node.get(e.e) else {
            return;
        };

        let Ok(Val::Number(x)) = fetch_port_data(
            "x",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut states.transmute_lens(),
            &mut tiles.as_query_lens(),
        ) else {
            error!("vector create: no or invalid x port");
            continue;
        };

        let Ok(Val::Number(y)) = fetch_port_data(
            "y",
            pos.0,
            &map,
            cfg,
            &mut metas.as_query_lens(),
            &mut states.transmute_lens(),
            &mut tiles.as_query_lens(),
        ) else {
            error!("vector create: no or invalid y port");
            continue;
        };

        let mut state = states.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Vec(vec2(x, y))));
    }
}

fn numsub_tick(
    mut evt: TickEvts<NumberSub>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut states: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
) {
    for e in evt.read() {
        info!("ticking number subtract");
        let Ok((pos, cfg)) = node.get(e.e) else {
            return;
        };
        let ports = ["a", "b", "c", "d", "e"];
        let res = ports
            .iter()
            .flat_map(|p| {
                fetch_port_data(
                    p,
                    pos.0,
                    &map,
                    cfg,
                    &mut metas.as_query_lens(),
                    &mut states.transmute_lens(),
                    &mut tiles.as_query_lens(),
                )
                .ok()
            })
            .flat_map(|v| match v {
                Val::Number(x) => Some(x),
                _ => None,
            })
            .reduce(|acc, v| acc - v)
            .unwrap_or(0.);

        let mut state = states.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Number(res)));
    }
}

fn nummul_tick(
    mut evt: TickEvts<NumberMul>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut states: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
) {
    for e in evt.read() {
        info!("ticking number multiply");
        let Ok((pos, cfg)) = node.get(e.e) else {
            return;
        };
        let ports = ["a", "b", "c", "d", "e"];
        let res = ports
            .iter()
            .flat_map(|p| {
                fetch_port_data(
                    p,
                    pos.0,
                    &map,
                    cfg,
                    &mut metas.as_query_lens(),
                    &mut states.transmute_lens(),
                    &mut tiles.as_query_lens(),
                )
                .ok()
            })
            .flat_map(|v| match v {
                Val::Number(x) => Some(x),
                _ => None,
            })
            .reduce(|acc, v| acc * v)
            .unwrap_or(0.);

        let mut state = states.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Number(res)));
    }
}

fn vecmul_tick(
    mut evt: TickEvts<VectorMul>,
    map: Res<Map>,
    node: Query<(&HexPos, &PortCfg)>,
    mut tiles: Query<&TileType>,
    mut states: Query<&mut CyberState>,
    mut metas: Query<&PortMeta>,
) {
    for e in evt.read() {
        info!("ticking vector multiply");
        let Ok((pos, cfg)) = node.get(e.e) else {
            return;
        };
        let ports = ["a", "b", "c", "d", "e"];
        let res = ports
            .iter()
            .flat_map(|p| {
                fetch_port_data(
                    p,
                    pos.0,
                    &map,
                    cfg,
                    &mut metas.as_query_lens(),
                    &mut states.transmute_lens(),
                    &mut tiles.as_query_lens(),
                )
                .ok()
            })
            .flat_map(|v| match v {
                Val::Vec(x) => Some(x),
                _ => None,
            })
            .reduce(|acc, v| acc * v)
            .unwrap_or(Vec2::splat(0.));

        let mut state = states.get_mut(e.e).unwrap();
        *state = CyberState::Done(Ok(Val::Vec(res)));
    }
}
