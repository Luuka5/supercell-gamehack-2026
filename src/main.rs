mod ai;
mod arena;
mod building;
mod combat;
mod logging;
mod pathfinding;
mod player;
mod user;

use ai::{AiPlayer, AiPlugin, AiRuleSet, PathFollower, TargetDestination};
use arena::areas::{Area, AreaID};
use arena::{ArenaConfig, ArenaDescription, ArenaPlugin, SpawnPoints};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use building::BuildingPlugin;
use building::StructureType;
use combat::{CombatPlugin, Enemy, Hp};
use logging::LoggingPlugin;
use player::{Inventory, MovementController, Player, PlayerPlugin, PlayerStatus};
use user::{MainCamera, SelectedBuildType, User, UserPlugin};

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Playing,
    GameOver,
}

// --- Game Constants ---

// Note: These are also defined in player.rs for now.
// Ideally, we should move them to a shared config resource.
const PLAYER_SIZE: Vec3 = Vec3::new(1.0, 3.0, 1.0);

// . = Floor
// X = Wall
// O = Obstacle
// T = Turret Resource
// B = Block (Obstacle) Resource
const ARENA_LAYOUT: &str = "
XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
X........X.............................X
X...T....X.............................X
X........X.............................X
X...XXXXXX.......XXXXXXXXXXXXXX....XXXXX
X...XXXXXX.......XXXXXXXXXXXXXX....XXXXX
X......................................X
X......................................X
X......................................X
X......................................X
X......................................X
X......................................X
X...XXXXXXXXXXXX...XXXX...XXXXXXXXXXXX.X
X...XXXXXXXXXXXX...XXXX...XXXXXXXXXXXX.X
X..................XXXX................X
X..................XXXX................X
X......................................X
X......................................X
X......................................X
X......................................X
X...XXXXXX.......XXXXXXXXXXXXXX....XXXXX
X...XXXXXX.......XXXXXXXXXXXXXX....XXXXX
X......................................X
X.............................X........X
X.............................X....T...X
X.............................X........X
XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
";

fn main() {
    let arena_description = ArenaDescription {
        layout: ARENA_LAYOUT.to_string(),
        areas: vec![
            Area::new(AreaID("UserBase".to_string()), 0, 0, 10, 10),
            Area::new(AreaID("EnemyBase".to_string()), 30, 0, 39, 10),
            Area::new(AreaID("CenterArena".to_string()), 11, 0, 29, 27),
            Area::new(AreaID("NorthCorridor".to_string()), 0, 11, 39, 27),
        ],
        player_spawn: Vec3::new(18.0, PLAYER_SIZE.y / 2.0, 10.0),
        ai_spawn: Vec3::new(26.0, PLAYER_SIZE.y / 2.0, 10.0),
        enemy_spawn: Vec3::new(142.0, PLAYER_SIZE.y / 2.0, 10.0),
        resource_respawn_time: 30.0,
    };

    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .add_plugins(ArenaPlugin::new(arena_description))
        .add_plugins(PlayerPlugin)
        .add_plugins(UserPlugin)
        .add_plugins(AiPlugin)
        .add_plugins(BuildingPlugin)
        .add_plugins(CombatPlugin)
        .add_plugins(LoggingPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (grab_cursor, debug_log_positions, draw_tile_grid))
        .run();
}

fn debug_log_positions(
    time: Res<Time>,
    query: Query<(Entity, &Transform), With<AiPlayer>>,
    mut timer: Local<f32>,
    config: Res<ArenaConfig>,
) {
    *timer += time.delta_secs();
    if *timer > 2.0 {
        *timer = 0.0;
    }
}

fn draw_tile_grid(
    mut gizmos: Gizmos,
    config: Res<ArenaConfig>,
    player_query: Query<&Transform, With<User>>,
    ghost_query: Query<&Transform, With<crate::building::BuildGhost>>,
) {
    for x in 0..=config.width {
        let world_x = x as f32 * config.tile_size;
        gizmos.line(
            Vec3::new(world_x, 0.05, 0.0),
            Vec3::new(world_x, 0.05, config.height as f32 * config.tile_size),
            Color::srgba(0.2, 0.8, 0.2, 0.5),
        );
    }

    for y in 0..=config.height {
        let world_z = y as f32 * config.tile_size;
        gizmos.line(
            Vec3::new(0.0, 0.05, world_z),
            Vec3::new(config.width as f32 * config.tile_size, 0.05, world_z),
            Color::srgba(0.2, 0.8, 0.2, 0.5),
        );
    }

    if let Ok(player_transform) = player_query.single() {
        let forward = player_transform.forward();
        let start = player_transform.translation;
        let end = start + forward * 3.0;
        gizmos.line(start, end, Color::srgb(1.0, 0.0, 0.0));
    }

    if let Ok(ghost_transform) = ghost_query.single() {
        let pos = ghost_transform.translation;
        gizmos.sphere(pos + Vec3::Y * 0.5, 0.2, Color::srgb(1.0, 1.0, 0.0));
    }
}

fn grab_cursor(
    mut q_windows: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
) {
    if let Some(mut cursor_options) = q_windows.iter_mut().next() {
        if mouse_btn.just_pressed(MouseButton::Left) {
            cursor_options.visible = false;
            cursor_options.grab_mode = CursorGrabMode::Locked;
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    spawn_points: Res<SpawnPoints>,
) {
    // Player (User controlled)
    commands.spawn((
        User,
        Player,
        Name::new("User"),
        PlayerStatus::default(),
        Inventory {
            obstacles: 6,
            turrets: 4,
        },
        MovementController::default(),
        SelectedBuildType(StructureType::Obstacle),
        Hp::new(3),
        Mesh3d(meshes.add(Cuboid::new(PLAYER_SIZE.x, PLAYER_SIZE.y, PLAYER_SIZE.z))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_translation(spawn_points.player),
    ));

    // AI Player
    /*
    commands.spawn((
        AiPlayer,
        Player,
        Name::new("Friendly AI"),
        PlayerStatus::default(),
        Inventory::default(),
        MovementController::default(),
        PathFollower::default(),
        TargetDestination { x: 35, y: 25 }, // Go to bottom right (Valid Y)
        AiRuleSet(ai::rules::RuleSet::default()),
        Hp::new(3),
        Mesh3d(meshes.add(Cuboid::new(PLAYER_SIZE.x, PLAYER_SIZE.y, PLAYER_SIZE.z))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.2, 0.8))), // Blue AI
        Transform::from_translation(spawn_points.ai),
    ));
    */

    // Enemy
    commands.spawn((
        AiPlayer,
        Enemy,
        Player,
        Name::new("Enemy"),
        PlayerStatus::default(),
        Inventory {
            obstacles: 6,
            turrets: 4,
        },
        MovementController::default(),
        PathFollower::default(),
        TargetDestination { x: 4, y: 2 }, // Go to User's start (Grid 4, 2)
        AiRuleSet(ai::rules::RuleSet::new_turret_only()),
        Hp::new(3),
        Mesh3d(meshes.add(Cuboid::new(PLAYER_SIZE.x, PLAYER_SIZE.y, PLAYER_SIZE.z))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))), // Red Enemy
        Transform::from_translation(spawn_points.enemy),
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: 3000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(80.0, 50.0, 54.0).looking_at(Vec3::new(80.0, 0.0, 54.0), Vec3::Y),
    ));

    // Camera
    commands.spawn((
        MainCamera { pitch: 0.3 },
        Camera3d::default(),
        Transform::from_xyz(16.0, 5.0, 26.0).looking_at(Vec3::new(16.0, 0.0, 16.0), Vec3::Y),
    ));
}
