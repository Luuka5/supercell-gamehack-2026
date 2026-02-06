mod arena;

use arena::ArenaPlugin;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

// --- Game Constants ---
const CAMERA_DISTANCE: f32 = 6.0;
const CAMERA_HEIGHT_OFFSET: f32 = 2.0;
const MOUSE_SENSITIVITY: f32 = 0.003;
const PLAYER_SPEED: f32 = 10.0;
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
X.......OOOOOOOO..........OOOOOOOO.....X
X.......OOOOOOOO..........OOOOOOOO.....X
X......................................X
X......................................X
X...XXXXXXXXXXXX...XXXX...XXXXXXXXXXXX.X
X...XXXXXXXXXXXX...XXXX...XXXXXXXXXXXX.X
X..................XXXX................X
X..................XXXX................X
X.......OOOOOOOO..........OOOOOOOO.....X
X.......OOOOOOOO..........OOOOOOOO.....X
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
        .add_systems(Startup, (setup, setup_cursor))
        .add_systems(
            Update,
            (move_player, player_look, camera_follow, grab_cursor),
        )
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct MainCamera {
    pitch: f32,
}

fn setup_cursor(mut q_windows: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>) {
    if let Some((_, mut cursor_options)) = q_windows.iter_mut().next() {
        cursor_options.visible = false;
        cursor_options.grab_mode = CursorGrabMode::Locked;
    }
}

fn grab_cursor(mut q_windows: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>) {
    if let Some((mut window, _)) = q_windows.iter_mut().next() {
        // Center the cursor
        let width = window.width();
        let height = window.height();
        window.set_cursor_position(Some(Vec2::new(width / 2.0, height / 2.0)));
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Player
    commands.spawn((
        Player,
        Mesh3d(meshes.add(Cuboid::new(PLAYER_SIZE.x, PLAYER_SIZE.y, PLAYER_SIZE.z))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(16.0, PLAYER_SIZE.y / 2.0, 16.0), // Start in the top-left base
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

fn player_look(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut camera_query: Query<&mut MainCamera>,
) {
    let delta = accumulated_mouse_motion.delta;

    if delta == Vec2::ZERO {
        return;
    }

    // Rotate Player (Yaw)
    if let Some(mut player_transform) = player_query.iter_mut().next() {
        player_transform.rotate_y(-delta.x * MOUSE_SENSITIVITY);
    }

    // Adjust Camera Pitch
    if let Some(mut camera) = camera_query.iter_mut().next() {
        camera.pitch += delta.y * MOUSE_SENSITIVITY;
        camera.pitch = camera.pitch.clamp(-1.5, 1.5);
    }
}

fn move_player(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    if let Some(mut transform) = query.iter_mut().next() {
        let mut direction = Vec3::ZERO;

        // Use local axes for movement
        let forward = transform.forward().as_vec3();
        let right = transform.right().as_vec3();

        if keyboard_input.pressed(KeyCode::KeyW) {
            direction += forward;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            direction -= forward;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            direction += right;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            direction -= right;
        }

        if direction.length_squared() > 0.0 {
            direction = direction.normalize();
            transform.translation += direction * PLAYER_SPEED * time.delta_secs();
        }
    }
}

fn camera_follow(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<(&mut Transform, &MainCamera), Without<Player>>,
) {
    if let Some(player_transform) = player_query.iter().next() {
        if let Some((mut camera_transform, camera)) = camera_query.iter_mut().next() {
            let look_target =
                player_transform.translation + Vec3::new(0.0, CAMERA_HEIGHT_OFFSET, 0.0);

            let pitch_rot = Quat::from_rotation_x(-camera.pitch);
            let rotation = player_transform.rotation * pitch_rot;
            let offset = rotation * Vec3::Z * CAMERA_DISTANCE;

            camera_transform.translation = look_target + offset;
            camera_transform.look_at(look_target, Vec3::Y);
        }
    }
}
