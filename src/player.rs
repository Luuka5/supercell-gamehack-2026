use crate::arena::{ArenaConfig, ArenaGrid, Collectible, CollectibleType};
use crate::pathfinding::has_line_of_sight;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

#[derive(Component, Default, Clone, Copy, PartialEq)]
pub enum BuildType {
    #[default]
    Obstacle,
    Turret,
}

#[derive(Component)]
pub struct SelectedBuildType(pub BuildType);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_user_input,
                execute_movement,
                camera_follow,
                update_player_visibility,
                update_inventory,
                handle_build_type_selection,
                update_hud_counts,
                update_hud_highlight,
            ),
        )
        .add_systems(Startup, setup_hud);
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

#[derive(Component, Default)]
pub struct PlayerStatus {
    pub visible_players: Vec<Entity>,
}

#[derive(Component, Default)]
pub struct Inventory {
    pub obstacles: u32,
    pub turrets: u32,
}

// Constants
const CAMERA_DISTANCE: f32 = 6.0;
const CAMERA_HEIGHT_OFFSET: f32 = 2.0;
const MOUSE_SENSITIVITY: f32 = 0.05;
const PLAYER_SPEED: f32 = 20.0;

fn update_player_visibility(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<Collectible>)>,
    config: Res<ArenaConfig>,
    grid: Res<ArenaGrid>,
) {
    let players: Vec<(Entity, Vec3)> = player_query
        .iter()
        .map(|(e, t)| (e, t.translation))
        .collect();

    for (entity, pos) in &players {
        let mut visible = Vec::new();

        for (other_entity, other_pos) in &players {
            if *entity == *other_entity {
                continue;
            }

            if has_line_of_sight(*pos, *other_pos, &config, &grid) {
                visible.push(*other_entity);
            }
        }

        // We blindly insert/overwrite for simplicity in this update loop.
        commands.entity(*entity).insert(PlayerStatus {
            visible_players: visible,
        });
    }
}

fn update_inventory(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<Collectible>)>,
    mut inventory_query: Query<&mut Inventory, With<Player>>,
    collectible_query: Query<(Entity, &Transform, &Collectible), Without<Player>>,
) {
    let players: Vec<(Entity, Vec3)> = player_query
        .iter()
        .map(|(e, t)| (e, t.translation))
        .collect();
    let collectibles: Vec<(Entity, Vec3, CollectibleType)> = collectible_query
        .iter()
        .map(|(e, t, c)| (e, t.translation, c.ty))
        .collect();

    for (player_entity, player_pos) in &players {
        let mut collected_entities = Vec::new();
        let mut collected_obstacles = 0;
        let mut collected_turrets = 0;

        for (collectible_entity, collectible_pos, ty) in &collectibles {
            if player_pos.distance(*collectible_pos) < 2.0 {
                collected_entities.push(*collectible_entity);
                match ty {
                    CollectibleType::Obstacle => collected_obstacles += 1,
                    CollectibleType::Turret => collected_turrets += 1,
                }
            }
        }

        if !collected_entities.is_empty() {
            for entity in collected_entities {
                commands.entity(entity).despawn();
            }

            if let Ok(mut inventory) = inventory_query.get_mut(*player_entity) {
                inventory.obstacles += collected_obstacles;
                inventory.turrets += collected_turrets;
                info!(
                    "Collected! Obstacles: {}, Turrets: {}",
                    inventory.obstacles, inventory.turrets
                );
            }
        }
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
    mut query: Query<(&mut Transform, &MovementController), (With<Player>, Without<Collectible>)>,
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
    player_query: Query<&Transform, (With<User>, Without<Collectible>)>,
    mut camera_query: Query<(&mut Transform, &MainCamera), (Without<User>, Without<Collectible>)>,
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
) {
    let mut selected = if let Ok(s) = selected_query.single_mut() {
        s
    } else {
        return;
    };

    if keyboard_input.just_pressed(KeyCode::Digit1) {
        *selected = SelectedBuildType(BuildType::Obstacle);
        info!("Selected: Obstacle (key 1)");
        return;
    }

    if keyboard_input.just_pressed(KeyCode::Digit2) {
        *selected = SelectedBuildType(BuildType::Turret);
        info!("Selected: Turret (key 2)");
        return;
    }

    for (interaction, text) in interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            if text.0.contains("Obstacle") {
                *selected = SelectedBuildType(BuildType::Obstacle);
                info!("Selected: Obstacle");
            } else if text.0.contains("Turret") {
                *selected = SelectedBuildType(BuildType::Turret);
                info!("Selected: Turret");
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
            BuildType::Obstacle => {
                *bg = BackgroundColor(Color::srgba(0.4, 0.3, 0.2, 1.0));
            }
            BuildType::Turret => {
                *bg = BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 1.0));
            }
        }
    }

    for mut bg in turret_btn_query.iter_mut() {
        match selected.0 {
            BuildType::Obstacle => {
                *bg = BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 1.0));
            }
            BuildType::Turret => {
                *bg = BackgroundColor(Color::srgba(0.2, 0.3, 0.4, 1.0));
            }
        }
    }
}
