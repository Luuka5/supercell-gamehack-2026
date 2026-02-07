mod ai;
mod arena;
mod building;
mod pathfinding;
mod player;

use ai::{AiPlayer, AiPlugin, Enemy, PathFollower, TargetDestination};
use arena::{ArenaConfig, ArenaPlugin};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use building::BuildingPlugin;
use player::{MainCamera, MovementController, Player, PlayerPlugin, User};

// --- Game Constants ---
// Note: These are also defined in player.rs for now.
// Ideally, we should move them to a shared config resource.
const PLAYER_SIZE: Vec3 = Vec3::new(1.0, 2.0, 1.0);

// . = Floor
// X = Wall
// O = Obstacle
const ARENA_LAYOUT: &str = "
XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
X........X.............................X
X........X.............................X
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
X.............................X........X
X.............................X........X
XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ArenaPlugin::new(ARENA_LAYOUT))
        .add_plugins(PlayerPlugin)
        .add_plugins(AiPlugin)
        .add_plugins(BuildingPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (grab_cursor, debug_log_positions))
        .run();
}

fn debug_log_positions(
    time: Res<Time>,
    query: Query<(Entity, &Transform), With<AiPlayer>>,
    mut timer: Local<f32>,
) {
    *timer += time.delta_secs();
    if *timer > 2.0 {
        *timer = 0.0;
        for (entity, transform) in query.iter() {
            info!("Entity {:?} at {}", entity, transform.translation);
        }
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
) {
    // Player (User controlled)
    commands.spawn((
        User,
        Player,
        MovementController::default(),
        Mesh3d(meshes.add(Cuboid::new(PLAYER_SIZE.x, PLAYER_SIZE.y, PLAYER_SIZE.z))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(16.0, PLAYER_SIZE.y / 2.0, 8.0), // Start in the top-left base (Grid 4, 2)
    ));

    // AI Player
    commands.spawn((
        AiPlayer,
        Player,
        MovementController::default(),
        PathFollower::default(),
        TargetDestination { x: 35, y: 25 }, // Go to bottom right (Valid Y)
        Mesh3d(meshes.add(Cuboid::new(PLAYER_SIZE.x, PLAYER_SIZE.y, PLAYER_SIZE.z))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.2, 0.8))), // Blue AI
        Transform::from_xyz(24.0, PLAYER_SIZE.y / 2.0, 8.0),       // Start near user (Grid 6, 2)
    ));

    // Enemy
    commands.spawn((
        AiPlayer,
        Enemy,
        Player,
        MovementController::default(),
        PathFollower::default(),
        TargetDestination { x: 4, y: 2 }, // Go to User's start (Grid 4, 2)
        Mesh3d(meshes.add(Cuboid::new(PLAYER_SIZE.x, PLAYER_SIZE.y, PLAYER_SIZE.z))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))), // Red Enemy
        Transform::from_xyz(140.0, PLAYER_SIZE.y / 2.0, 8.0),      // Start far away (Grid 35, 2)
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
