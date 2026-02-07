use crate::arena::Collectible;
use crate::player::{BuildType, Inventory, MainCamera, MovementController, SelectedBuildType};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

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
