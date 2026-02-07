use crate::arena::{ArenaConfig, ArenaGrid, Collectible};
use crate::player::{
    Inventory, MovementController, SelectedBuildType, Structure, StructureType, TurretDirection,
};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

const CAMERA_DISTANCE: f32 = 6.0;
const CAMERA_HEIGHT_OFFSET: f32 = 1.;
const CAMERA_ROTATING_HEIGHT_OFFSET: f32 = 0.5;
const CAMERA_MIN_DISTANCE: f32 = 1.5;
const CAMERA_DISTANCE_MARGIN: f32 = 0.1;

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
pub struct TurretCountText;

#[derive(Component)]
pub struct HpBarBackground;

#[derive(Component)]
pub struct HpBarFill;

#[derive(Component)]
pub struct MainCamera {
    pub pitch: f32,
}

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
                update_hp_bar,
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

    commands.spawn((
        HudContainer,
        Text::new("HP:"),
        TextFont::from_font_size(18.0),
        TextColor(Color::WHITE),
        ChildOf(panel),
    ));

    let hp_bar_bg = commands
        .spawn((
            HudContainer,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 1.0)),
            BorderColor::all(Color::srgba(0.5, 0.5, 0.5, 1.0)),
            ChildOf(panel),
        ))
        .id();

    commands.spawn((
        HudContainer,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.0, 0.8, 0.2)),
        HpBarFill,
        ChildOf(hp_bar_bg),
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
    config: Res<ArenaConfig>,
    grid: Res<ArenaGrid>,
    structure_query: Query<&Structure>,
) {
    if let Some(player_transform) = player_query.iter().next() {
        if let Some((mut camera_transform, camera)) = camera_query.iter_mut().next() {
            let look_target =
                player_transform.translation + Vec3::new(0.0, CAMERA_HEIGHT_OFFSET, 0.0);

            let pitch_rot = Quat::from_rotation_x(-camera.pitch);
            let rotation = player_transform.rotation * pitch_rot;

            let camera_dir = rotation * Vec3::Z;
            let desired_distance = get_collision_adjusted_distance(
                look_target,
                camera_dir,
                CAMERA_DISTANCE,
                CAMERA_MIN_DISTANCE,
                CAMERA_DISTANCE_MARGIN,
                &config,
                &grid,
                &structure_query,
            );

            let offset = rotation * (Vec3::Z * desired_distance + CAMERA_ROTATING_HEIGHT_OFFSET);

            camera_transform.translation = look_target + offset;
            camera_transform.look_at(look_target, Vec3::Y);
        }
    }
}

fn get_collision_adjusted_distance(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    min_distance: f32,
    distance_margin: f32,
    config: &ArenaConfig,
    grid: &ArenaGrid,
    structure_query: &Query<&Structure>,
) -> f32 {
    let mut closest_hit = max_distance;

    for (&key, &entity) in grid.occupants.iter() {
        let (tile_x, tile_z) = key;
        let Ok(structure) = structure_query.get(entity) else {
            continue;
        };

        let half_size = config.tile_size * 0.5 * structure.collider_scale;
        let center_x = tile_x as f32 * config.tile_size + config.tile_size * 0.5;
        let center_z = tile_z as f32 * config.tile_size + config.tile_size * 0.5;

        let min = Vec3::new(center_x - half_size, 0.0, center_z - half_size);
        let max = Vec3::new(center_x + half_size, 8.0, center_z + half_size);

        if let Some(distance) = ray_aabb_intersection(origin, direction, min, max) {
            if distance < closest_hit && distance > min_distance {
                closest_hit = distance - distance_margin;
            }
        }
    }

    closest_hit.max(min_distance)
}

fn ray_aabb_intersection(origin: Vec3, direction: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let inv_dir = Vec3::new(
        if direction.x != 0.0 {
            1.0 / direction.x
        } else {
            f32::MAX
        },
        if direction.y != 0.0 {
            1.0 / direction.y
        } else {
            f32::MAX
        },
        if direction.z != 0.0 {
            1.0 / direction.z
        } else {
            f32::MAX
        },
    );

    let t1 = (min.x - origin.x) * inv_dir.x;
    let t2 = (max.x - origin.x) * inv_dir.x;
    let t3 = (min.y - origin.y) * inv_dir.y;
    let t4 = (max.y - origin.y) * inv_dir.y;
    let t5 = (min.z - origin.z) * inv_dir.z;
    let t6 = (max.z - origin.z) * inv_dir.z;

    let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
    let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

    if tmax < 0.0 || tmin > tmax {
        return None;
    }

    if tmin < 0.0 {
        Some(tmax)
    } else {
        Some(tmin)
    }
}

fn update_hp_bar(
    mut hp_bar_query: Query<&mut Node, With<HpBarFill>>,
    player_query: Query<&crate::player::Hp, With<User>>,
) {
    if let Ok(hp) = player_query.single() {
        for mut node in hp_bar_query.iter_mut() {
            let percentage = hp.current as f32 / hp.max as f32;
            node.width = Val::Percent(percentage * 100.0);
        }
    }
}
