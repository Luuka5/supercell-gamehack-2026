use crate::arena::Collectible;
use crate::player::{
    Inventory, MainCamera, MovementController, SelectedBuildType, StructureType, TurretDirection,
};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

const CAMERA_DISTANCE: f32 = 6.0;
const CAMERA_HEIGHT_OFFSET: f32 = 2.0;

#[derive(Component)]
pub struct User;

#[derive(Component)]
struct HudContainer;

#[derive(Component)]
struct ObstacleButton;

#[derive(Component)]
struct TurretButton;

#[derive(Component)]
struct ObstacleCountText;

#[derive(Component)]
struct TurretCountText;

pub struct UserPlugin;

impl Plugin for UserPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_user_input,
                camera_follow,
                handle_build_type_selection,
                update_hud_counts,
                update_hud_highlight,
            ),
        )
        .add_systems(Startup, setup_hud);
    }
}

fn handle_user_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    mut query: Query<&mut MovementController, (With<User>, Without<Collectible>)>,
    mut camera_query: Query<&mut MainCamera, Without<Collectible>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    let delta = accumulated_mouse_motion.delta;

    let rotation_scale = if let Some(window) = window_query.iter().next() {
        let width = window.width();
        let height = window.height();
        0.0005 * height.min(width)
    } else {
        1.0
    };

    if delta != Vec2::ZERO {
        if let Some(mut camera) = camera_query.iter_mut().next() {
            camera.pitch += delta.y * rotation_scale * 0.05;
            camera.pitch = camera.pitch.clamp(-1.5, 1.5);
        }
    }

    for mut controller in query.iter_mut() {
        if delta.x != 0.0 {
            controller.rotation_delta = -delta.x * rotation_scale * 0.05;
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

fn setup_hud(mut commands: Commands) {
    let container = commands
        .spawn((
            HudContainer,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .id();

    let panel = commands
        .spawn((
            HudContainer,
            Node {
                width: Val::Px(220.0),
                height: Val::Px(140.0),
                left: Val::Px(20.0),
                top: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
            BorderColor::all(Color::srgba(0.5, 0.5, 0.5, 1.0)),
            ChildOf(container),
        ))
        .id();

    commands.spawn((
        HudContainer,
        Text::new("BUILD SELECTION"),
        TextFont::from_font_size(18.0),
        TextColor(Color::WHITE),
        ChildOf(panel),
    ));

    let obstacle_btn = commands
        .spawn((
            HudContainer,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(30.0),
                display: Display::Flex,
                column_gap: Val::Px(10.0),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::all(Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 1.0)),
            BorderColor::all(Color::srgba(0.4, 0.4, 0.4, 1.0)),
            Interaction::None,
            ObstacleButton,
            ChildOf(panel),
        ))
        .id();

    commands.spawn((
        Text::new("Obstacle"),
        TextFont::from_font_size(16.0),
        TextColor(Color::WHITE),
        ChildOf(obstacle_btn),
    ));

    commands.spawn((
        Text::new("x0"),
        TextFont::from_font_size(16.0),
        TextColor(Color::WHITE),
        ObstacleCountText,
        ChildOf(obstacle_btn),
    ));

    let turret_btn = commands
        .spawn((
            HudContainer,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(30.0),
                display: Display::Flex,
                column_gap: Val::Px(10.0),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::all(Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 1.0)),
            BorderColor::all(Color::srgba(0.4, 0.4, 0.4, 1.0)),
            Interaction::None,
            TurretButton,
            ChildOf(panel),
        ))
        .id();

    commands.spawn((
        Text::new("Turret"),
        TextFont::from_font_size(16.0),
        TextColor(Color::WHITE),
        ChildOf(turret_btn),
    ));

    commands.spawn((
        Text::new("x0"),
        TextFont::from_font_size(16.0),
        TextColor(Color::WHITE),
        TurretCountText,
        ChildOf(turret_btn),
    ));
}

fn handle_build_type_selection(
    mut interaction_query: Query<
        (&Interaction, &Text),
        (
            Or<(With<ObstacleButton>, With<TurretButton>)>,
            Changed<Interaction>,
        ),
    >,
    mut selected_query: Query<&mut SelectedBuildType, With<User>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    player_query: Query<&Transform, With<User>>,
) {
    let mut selected = if let Ok(s) = selected_query.single_mut() {
        s
    } else {
        return;
    };

    let player_transform = if let Ok(t) = player_query.single() {
        t
    } else {
        return;
    };

    let forward = player_transform.forward();
    let abs_x = forward.x.abs();
    let abs_z = forward.z.abs();
    let turret_direction = if abs_x > abs_z {
        if forward.x > 0.0 {
            TurretDirection::East
        } else {
            TurretDirection::West
        }
    } else {
        if forward.z > 0.0 {
            TurretDirection::South
        } else {
            TurretDirection::North
        }
    };

    if keyboard_input.just_pressed(KeyCode::Digit1) {
        *selected = SelectedBuildType(StructureType::Obstacle);
        info!("Selected: Obstacle (key 1)");
        return;
    }

    if keyboard_input.just_pressed(KeyCode::Digit2) {
        *selected = SelectedBuildType(StructureType::Turret(turret_direction));
        info!(
            "Selected: Turret (key 2) facing {:?} (forward: {:?})",
            turret_direction, forward
        );
        return;
    }

    for (interaction, text) in interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            if text.0.contains("Obstacle") {
                *selected = SelectedBuildType(StructureType::Obstacle);
                info!("Selected: Obstacle");
            } else if text.0.contains("Turret") {
                *selected = SelectedBuildType(StructureType::Turret(turret_direction));
                info!(
                    "Selected: Turret facing {:?} (forward: {:?})",
                    turret_direction, forward
                );
            }
        }
    }
}

fn update_hud_counts(
    mut obstacle_count_query: Query<&mut Text, (With<ObstacleCountText>, Without<TurretCountText>)>,
    mut turret_count_query: Query<&mut Text, (With<TurretCountText>, Without<ObstacleCountText>)>,
    inventory_query: Query<&Inventory, With<User>>,
) {
    if let Ok(inventory) = inventory_query.single() {
        for mut text in obstacle_count_query.iter_mut() {
            text.0 = format!("x{}", inventory.obstacles);
        }
        for mut text in turret_count_query.iter_mut() {
            text.0 = format!("x{}", inventory.turrets);
        }
    }
}

fn update_hud_highlight(
    mut obstacle_btn_query: Query<
        &mut BackgroundColor,
        (With<ObstacleButton>, Without<TurretButton>),
    >,
    mut turret_btn_query: Query<
        &mut BackgroundColor,
        (With<TurretButton>, Without<ObstacleButton>),
    >,
    selected_query: Query<&SelectedBuildType, With<User>>,
) {
    let selected = if let Ok(s) = selected_query.single() {
        s
    } else {
        return;
    };

    for mut bg in obstacle_btn_query.iter_mut() {
        match selected.0 {
            StructureType::Obstacle => {
                *bg = BackgroundColor(Color::srgba(0.4, 0.3, 0.2, 1.0));
            }
            StructureType::Turret(_) => {
                *bg = BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 1.0));
            }
            StructureType::Wall => {
                *bg = BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 1.0));
            }
        }
    }

    for mut bg in turret_btn_query.iter_mut() {
        match selected.0 {
            StructureType::Obstacle => {
                *bg = BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 1.0));
            }
            StructureType::Turret(_) => {
                *bg = BackgroundColor(Color::srgba(0.2, 0.3, 0.4, 1.0));
            }
            StructureType::Wall => {
                *bg = BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 1.0));
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
