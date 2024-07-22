use bevy::{
    color::palettes::css::{BLUE, GREEN, RED},
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
use hexx::{algorithms, Hex};
use rand::{seq::IteratorRandom, Rng};

use crate::{
    nodes::{HexPos, TargetableEntity},
    Gamestate, Map, Tick, TileType,
};

pub struct EnemyPlugin;
impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_res).add_systems(
            Update,
            (spawner, follow_path, draw_path).run_if(in_state(Gamestate::Game)),
        );
    }
}

#[derive(Component)]
struct PathfindTarget(Entity);

#[derive(Component)]
struct PathfindPath {
    path: Vec<Hex>,
    i: usize,
}

#[derive(Bundle)]
struct EnemyBundle {
    apperance: SpriteBundle,
    target: PathfindTarget,
    path: PathfindPath,
    activity: EnemyActivity,
    targetable: TargetableEntity,
}

#[derive(Component, Default, PartialEq)]
enum EnemyActivity {
    #[default]
    Pathfinding,
    Attacking,
}

#[derive(Resource)]
struct EnemyRes {
    ball: Handle<Image>,
}

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

fn draw_path(mut gizmos: Gizmos, map: Res<Map>, mut ents: Query<(&Transform, &PathfindPath)>) {
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
        // 1/5 chance to spawn enemy
        if !rand::thread_rng().gen_bool(1. / 10.) {
            continue;
        }

        let Ok(heart) = hearts.get(t.0) else {
            // heart was propaby destroyed
            continue;
        };

        let mut rng = rand::thread_rng();
        let spawntile = heart.0.ring(8).choose(&mut rng).unwrap();
        let spawnpos = map.layout.hex_to_world_pos(spawntile);
        let Some(path) = algorithms::a_star(spawntile, heart.0, |_, h2| {
            let Some(e2) = map.storage.get(h2) else {
                return Some(0);
            };
            let t2 = types.get(*e2).unwrap();
            match t2 {
                TileType::Unoccupied => Some(0),
                TileType::Heart(_) => Some(1),
                TileType::Terrain => Some(10),
                TileType::CyberNode { .. } => Some(50),
            }
        }) else {
            // no path to target
            return;
        };

        cmd.spawn(EnemyBundle {
            apperance: SpriteBundle {
                texture: res.ball.clone(),
                transform: Transform::from_translation(spawnpos.extend(1.)),
                ..default()
            },
            target: PathfindTarget(t.0),
            path: PathfindPath { path, i: 0 },
            activity: EnemyActivity::default(),
            targetable: TargetableEntity,
        });
    }
}
