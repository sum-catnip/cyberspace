use std::time::Duration;

use bevy::{
    color::palettes::css::{BLUE, GREEN, RED},
    prelude::*,
};
use hexx::{algorithms, Hex};
use rand::{seq::IteratorRandom, Rng};

use crate::{
    nodes::{Health, HexPos, TargetableEntity},
    CommonResources, Debug, Gamestate, Map, Tick, TileType,
};

pub struct EnemyPlugin;
impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_res).add_systems(
            FixedUpdate,
            (
                spawner,
                attack,
                activity_transition,
                follow_path,
                scale_enemy,
                draw_path,
                despawn,
            )
                .run_if(in_state(Gamestate::Game)),
        );
    }
}

#[derive(Component)]
struct PathfindTarget(Entity);

#[derive(Component)]
struct Dmg(f32);

#[derive(Component)]
pub struct PathfindPath {
    pub path: Vec<Hex>,
    pub i: usize,
}

#[derive(Bundle)]
struct EnemyBundle {
    apperance: SpriteBundle,
    target: PathfindTarget,
    path: PathfindPath,
    activity: EnemyActivity,
    targetable: TargetableEntity,
    health: Health,
    dmg: Dmg,
}

#[derive(Component, Default, PartialEq)]
enum EnemyActivity {
    #[default]
    Pathfinding,
    Attacking(Entity, Timer),
}

#[derive(Resource)]
struct EnemyRes {
    ball: Handle<Image>,
}

const MAX_HP: f32 = 500.;

fn load_res(mut cmd: Commands, ass: Res<AssetServer>) {
    cmd.insert_resource(EnemyRes {
        ball: ass.load("enemy.png"),
    });
}

fn follow_path(
    map: Res<Map>,
    mut ents: Query<(&mut Transform, &EnemyActivity, &mut PathfindPath)>,
    time: Res<Time>,
) {
    for (mut trans, activity, mut path) in ents.iter_mut() {
        if *activity != EnemyActivity::Pathfinding {
            continue;
        };

        let Some(next) = path.path.get(path.i) else {
            // done
            continue;
        };

        let pos = trans.translation.xy();
        let nextpos = map.layout.hex_to_world_pos(*next);
        let diff = nextpos - pos;
        if diff.length() < 0.1 {
            path.i += 1;
            continue;
        }
        let dir = Dir2::new(diff).unwrap();
        let speed = 10.;

        trans.translation += (dir * speed).extend(0.) * time.delta_seconds();
    }
}

fn draw_path(
    mut gizmos: Gizmos,
    dbg: Res<Debug>,
    map: Res<Map>,
    mut ents: Query<(&Transform, &PathfindPath)>,
) {
    if !dbg.enemy_paths {
        return;
    };
    for (trans, path) in ents.iter_mut() {
        gizmos.circle_2d(trans.translation.xy(), 10., Color::from(RED));
        if let Some(next) = path.path.get(path.i) {
            gizmos.circle_2d(map.layout.hex_to_world_pos(*next), 10., Color::from(GREEN));
        }
        for parts in path.path.windows(2) {
            let [h1, h2] = parts else { unreachable!() };
            let p1 = map.layout.hex_to_world_pos(*h1);
            let p2 = map.layout.hex_to_world_pos(*h2);
            gizmos.line_2d(p1, p2, Color::from(BLUE));
        }
    }
}

fn attack(
    mut cmd: Commands,
    time: Res<Time>,
    common: Res<CommonResources>,
    mut health: Query<&mut Health>,
    mut ents: Query<(&mut EnemyActivity, &Dmg)>,
) {
    for (mut activity, dmg) in ents.iter_mut() {
        let EnemyActivity::Attacking(e, t) = activity.as_mut() else {
            continue;
        };

        if !t.tick(time.delta()).just_finished() {
            continue;
        };

        let Ok(mut hp) = health.get_mut(*e) else {
            warn!("tried attacking entity that no longer exists or has no hp");
            continue;
        };

        hp.0 -= dmg.0;
        info!("attacking with {:?} dmg. new hp: {:?}", dmg.0, hp.0);
        cmd.spawn(AudioBundle {
            source: common.dmg_sound.clone(),
            ..default()
        });
    }
}

fn scale_enemy(mut ents: Query<(&Health, &mut Dmg, &mut Transform)>) {
    for (hp, mut dmg, mut trans) in ents.iter_mut() {
        dmg.0 = f32::max(hp.0 / 10., 1.);
        trans.scale = Vec2::splat(hp.0 / MAX_HP / 2.).extend(0.);
    }
}

fn activity_transition(
    map: Res<Map>,
    tt: Query<&TileType>,
    mut ents: Query<(&mut EnemyActivity, &Transform)>,
) {
    for (mut activity, trans) in ents.iter_mut() {
        let pos = map.layout.world_pos_to_hex(trans.translation.xy());
        let Some(ne) = map.storage.get(pos) else {
            continue;
        };

        let nt = tt.get(*ne).unwrap();
        if *nt == TileType::Unoccupied {
            activity.set_if_neq(EnemyActivity::Pathfinding);
            continue;
        }

        let e = match nt {
            TileType::CyberNode { e, .. } => e,
            TileType::Terrain(e) => e,
            TileType::Heart(e) => e,
            _ => unreachable!(),
        };

        if !matches!(activity.as_ref(), EnemyActivity::Attacking(..)) {
            *activity = EnemyActivity::Attacking(
                *e,
                Timer::new(Duration::from_secs(1), TimerMode::Repeating),
            );
        }
    }
}

// spawn in radius around hearts
fn spawner(
    mut cmd: Commands,
    mut ticks: EventReader<Tick>,
    res: Res<EnemyRes>,
    map: Res<Map>,
    types: Query<&TileType>,
    hearts: Query<&HexPos>,
) {
    for t in ticks.read() {
        // 1/3 chance to spawn enemy
        if !rand::thread_rng().gen_bool(1. / 3.) {
            continue;
        }

        let Ok(heart) = hearts.get(t.0) else {
            // heart was propaby destroyed
            continue;
        };

        let mut rng = rand::thread_rng();
        let spawntile = heart.0.ring(15).choose(&mut rng).unwrap();
        let spawnpos = map.layout.hex_to_world_pos(spawntile);
        let Some(path) = algorithms::a_star(spawntile, heart.0, |_, h2| {
            let Some(e2) = map.storage.get(h2) else {
                return Some(0);
            };
            let t2 = types.get(*e2).unwrap();
            match t2 {
                TileType::Unoccupied => Some(0),
                TileType::Heart(_) => Some(1),
                TileType::Terrain(_) => Some(10),
                TileType::CyberNode { .. } => Some(50),
            }
        }) else {
            // no path to target
            return;
        };

        cmd.spawn(EnemyBundle {
            apperance: SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(5., 1., 1.),
                    ..default()
                },
                texture: res.ball.clone(),
                transform: Transform::from_translation(spawnpos.extend(1.))
                    .with_scale(Vec3::splat(0.)),
                ..default()
            },
            target: PathfindTarget(t.0),
            path: PathfindPath { path, i: 0 },
            activity: EnemyActivity::default(),
            targetable: TargetableEntity,
            health: Health(rng.gen_range(10..MAX_HP as u32) as f32),
            dmg: Dmg(10.),
        });
    }
}

fn despawn(mut cmd: Commands, hp: Query<(Entity, &Health)>) {
    for (e, hp) in hp.iter() {
        if hp.0 <= 0. {
            cmd.entity(e).despawn_recursive();
        }
    }
}
