use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (handle_user_input, execute_movement, camera_follow));
    }
}

#[derive(Component)]
pub struct User;

#[derive(Component)]
pub struct Player;

#[derive(Component, Default)]
pub struct MovementController {
    pub input_direction: Vec3,
    pub rotation_delta: f32,
}

#[derive(Component)]
pub struct MainCamera {
    pub pitch: f32,
}

// Constants (duplicated from main for now, or should be shared?)
// Let's redefine them here or pass them via resource.
// For simplicity, I'll redefine them.
const CAMERA_DISTANCE: f32 = 6.0;
const CAMERA_HEIGHT_OFFSET: f32 = 2.0;
const MOUSE_SENSITIVITY: f32 = 0.05;
const PLAYER_SPEED: f32 = 20.0;

fn handle_user_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    mut query: Query<&mut MovementController, With<User>>,
    mut camera_query: Query<&mut MainCamera>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    let delta = accumulated_mouse_motion.delta;

    let rotation_scale = if let Some(window) = window_query.iter().next() {
        0.0005 * window.height().min(window.width())
    } else {
        1.0
    };

    if delta != Vec2::ZERO {
        if let Some(mut camera) = camera_query.iter_mut().next() {
            camera.pitch += delta.y * rotation_scale * MOUSE_SENSITIVITY;
            camera.pitch = camera.pitch.clamp(-1.5, 1.5);
        }
    }

    for mut controller in query.iter_mut() {
        if delta.x != 0.0 {
            controller.rotation_delta = -delta.x * rotation_scale * MOUSE_SENSITIVITY;
        } else {
            controller.rotation_delta = 0.0;
        }

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
        if controller.rotation_delta != 0.0 {
            transform.rotate_y(controller.rotation_delta);
        }

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
