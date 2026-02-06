mod arena;

use arena::{find_path, ArenaConfig, ArenaPlugin, NavGraph};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

// --- Game Constants ---
const CAMERA_DISTANCE: f32 = 6.0;
const CAMERA_HEIGHT_OFFSET: f32 = 2.0;
const MOUSE_SENSITIVITY: f32 = 0.05;
const PLAYER_SPEED: f32 = 20.0;
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
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                grab_cursor,
                handle_user_input,
                execute_movement,
                camera_follow,
                pathfinding_system,
                path_following_system,
                debug_log_positions,
            ),
        )
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
            println!("Entity {:?} at {}", entity, transform.translation);
        }
    }
}

#[derive(Component)]
struct User;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct AiPlayer;

#[derive(Component)]
struct Enemy;

#[derive(Component, Default)]
struct MovementController {
    pub input_direction: Vec3,
    pub rotation_delta: f32,
}

#[derive(Component)]
struct MainCamera {
    pitch: f32,
}

#[derive(Component)]
struct TargetDestination {
    pub x: u32,
    pub y: u32,
}

#[derive(Component, Default)]
struct PathFollower {
    pub path: Vec<(u32, u32)>,
    pub current_index: usize,
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

fn handle_user_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    mut query: Query<&mut MovementController, With<User>>,
    mut camera_query: Query<&mut MainCamera>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    let delta = accumulated_mouse_motion.delta;

    // Normalize sensitivity based on window height to handle different resolutions/DPI
    // We use the smaller dimension to ensure consistent feel on ultrawide monitors
    let rotation_scale = if let Some(window) = window_query.iter().next() {
        0.0005 * window.height().min(window.width())
    } else {
        1.0 // Fallback if no window found (shouldn't happen)
    };

    // Adjust Camera Pitch (User specific)
    if delta != Vec2::ZERO {
        if let Some(mut camera) = camera_query.iter_mut().next() {
            camera.pitch += delta.y * rotation_scale * MOUSE_SENSITIVITY;
            camera.pitch = camera.pitch.clamp(-1.5, 1.5);
        }
    }

    for mut controller in query.iter_mut() {
        // Rotation (Yaw)
        if delta.x != 0.0 {
            controller.rotation_delta = -delta.x * rotation_scale * MOUSE_SENSITIVITY;
        } else {
            controller.rotation_delta = 0.0;
        }

        // Movement
        let mut direction = Vec3::ZERO;
        if keyboard_input.pressed(KeyCode::KeyW) {
            direction += Vec3::Z;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            direction -= Vec3::Z;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            direction += Vec3::X;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            direction -= Vec3::X;
        }

        controller.input_direction = direction;
    }
}

fn execute_movement(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &MovementController), With<Player>>,
) {
    for (mut transform, controller) in query.iter_mut() {
        // Apply Rotation
        if controller.rotation_delta != 0.0 {
            transform.rotate_y(controller.rotation_delta);
        }

        // Apply Translation
        if controller.input_direction.length_squared() > 0.0 {
            let local_dir = transform.forward().as_vec3() * controller.input_direction.z
                + transform.right().as_vec3() * controller.input_direction.x;

            if local_dir.length_squared() > 0.0 {
                let move_dir = local_dir.normalize();
                transform.translation += move_dir * PLAYER_SPEED * time.delta_secs();
            }
        }
    }
}

fn camera_follow(
    player_query: Query<&Transform, With<User>>,
    mut camera_query: Query<(&mut Transform, &MainCamera), Without<User>>,
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

fn pathfinding_system(
    mut commands: Commands,
    mut query: Query<
        (Entity, &Transform, &TargetDestination),
        (With<AiPlayer>, Changed<TargetDestination>),
    >,
    nav_graph: Res<NavGraph>,
    config: Res<ArenaConfig>,
) {
    for (entity, transform, target) in query.iter_mut() {
        // Convert world position to grid coordinates
        let start_x = (transform.translation.x / config.tile_size).round() as u32;
        let start_y = (transform.translation.z / config.tile_size).round() as u32;

        if let Some(path) = find_path((start_x, start_y), (target.x, target.y), &nav_graph) {
            info!(
                "Path found for AI {:?}: {} steps from ({}, {}) to ({}, {})",
                entity,
                path.len(),
                start_x,
                start_y,
                target.x,
                target.y
            );
            commands.entity(entity).insert(PathFollower {
                path,
                current_index: 0,
            });
        } else {
            warn!(
                "No path found for AI {:?} from ({}, {}) to ({}, {})!",
                entity, start_x, start_y, target.x, target.y
            );
        }
    }
}

fn path_following_system(
    mut query: Query<
        (
            Entity,
            &Transform,
            &mut PathFollower,
            &mut MovementController,
        ),
        With<AiPlayer>,
    >,
    config: Res<ArenaConfig>,
) {
    for (entity, transform, mut follower, mut controller) in query.iter_mut() {
        if follower.current_index >= follower.path.len() {
            controller.input_direction = Vec3::ZERO;
            continue;
        }

        let target_node = follower.path[follower.current_index];
        let target_pos = Vec3::new(
            target_node.0 as f32 * config.tile_size,
            transform.translation.y,
            target_node.1 as f32 * config.tile_size,
        );

        let direction = target_pos - transform.translation;
        let distance = direction.length();

        if distance < 0.5 {
            follower.current_index += 1;
            info!(
                "AI {:?} reached node {:?}. Next: {:?}",
                entity, target_node, follower.current_index
            );
        } else {
            // Calculate movement direction relative to the AI's current rotation
            // We want to move towards the target.
            // The execute_movement system expects input_direction.z to be forward/backward and x to be right/left relative to the entity.
            // However, for AI, it's easier to just rotate towards the target and move "forward".

            // Simple AI movement: Rotate to face target, then move forward (Z)

            // Calculate desired yaw
            let forward = direction.normalize();
            let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();

            if flat_forward != Vec3::ZERO {
                // Calculate the local direction vector to the target.
                let local_direction = transform.rotation.inverse() * direction;

                // Normalize and map to input
                let move_dir = local_direction.normalize();

                // In our execute_movement:
                // local_dir = forward * input.z + right * input.x
                // forward is -Z, right is +X
                // so local_dir = -Z * input.z + X * input.x
                // input.z should be -move_dir.z
                // input.x should be move_dir.x

                controller.input_direction = Vec3::new(move_dir.x, 0.0, -move_dir.z);

                // Also rotate towards the target to look natural
                // We can use the X component of the local direction to steer
                controller.rotation_delta = -move_dir.x * 0.1; // Simple steering
            }
        }
    }
}
