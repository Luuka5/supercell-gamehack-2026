use bevy::prelude::*;

use crate::GameState;
use crate::user::User;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            turret_shooting_system.run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component)]
pub struct Hp {
    pub current: u32,
    pub max: u32,
}

impl Hp {
    pub fn new(max: u32) -> Self {
        Self { current: max, max }
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0
    }

    pub fn take_damage(&mut self, amount: u32) {
        self.current = self.current.saturating_sub(amount);
    }
}

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum TurretDirection {
    North,
    East,
    South,
    West,
}

impl TurretDirection {
    pub fn to_vec3(&self) -> Vec3 {
        match self {
            TurretDirection::North => -Vec3::Z,
            TurretDirection::East => Vec3::X,
            TurretDirection::South => Vec3::Z,
            TurretDirection::West => -Vec3::X,
        }
    }

    pub fn from_quat(rotation: Quat) -> Self {
        let forward = rotation * -Vec3::Z;
        let abs_x = forward.x.abs();
        let abs_z = forward.z.abs();

        if abs_x > abs_z {
            if forward.x > 0.0 {
                TurretDirection::East
            } else {
                TurretDirection::West
            }
        } else {
            if forward.z < 0.0 {
                TurretDirection::North
            } else {
                TurretDirection::South
            }
        }
    }

    pub fn to_quat(&self) -> Quat {
        match self {
            TurretDirection::North => Quat::IDENTITY,
            TurretDirection::East => Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
            TurretDirection::South => Quat::from_rotation_y(std::f32::consts::PI),
            TurretDirection::West => Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        }
    }
}

#[derive(Component)]
pub struct Turret {
    pub owner: Entity,
    pub direction: TurretDirection,
    pub last_shot: f32,
}

impl Turret {
    pub fn is_active(&self, current_time: f32) -> bool {
        // Hardcoded cooldown from player.rs was 4.0
        current_time - self.last_shot >= 4.0
    }

    pub fn shot(&mut self, current_time: f32) {
        self.last_shot = current_time;
    }
}

// Consolidating Enemy here
#[derive(Component)]
pub struct Enemy;

pub const TURRET_DAMAGE: u32 = 1;

fn turret_shooting_system(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    turret_query: Query<(Entity, &Transform, &Turret), Without<Enemy>>,
    mut enemy_query: Query<(Entity, &Transform, &mut Hp, Option<&User>), With<Enemy>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let current_time = time.elapsed_secs();

    for (turret_entity, turret_transform, turret) in turret_query.iter() {
        if !turret.is_active(current_time) {
            continue;
        }

        let turret_pos = turret_transform.translation;
        let direction_vec = turret.direction.to_vec3();

        let mut closest_enemy: Option<Entity> = None;
        let mut closest_distance = f32::MAX;

        for (enemy_entity, enemy_transform, _, _) in enemy_query.iter() {
            let enemy_pos = enemy_transform.translation;
            let to_enemy = enemy_pos - turret_pos;
            let distance = to_enemy.length();

            if distance < 15.0 {
                let enemy_dir = to_enemy.normalize();
                let dot = enemy_dir.dot(direction_vec);

                if dot > 0.95 {
                    if distance < closest_distance {
                        closest_distance = distance;
                        closest_enemy = Some(enemy_entity);
                    }
                }
            }
        }

        if let Some(enemy_entity) = closest_enemy {
            commands.entity(turret_entity).insert(Turret {
                owner: turret.owner,
                direction: turret.direction,
                last_shot: current_time,
            });

            if let Ok((enemy_entity, _enemy_transform, mut hp, user)) =
                enemy_query.get_mut(enemy_entity)
            {
                hp.take_damage(TURRET_DAMAGE);
                if hp.is_alive() {
                    info!(
                        "Turret hit {:?}! HP: {}/{}",
                        enemy_entity, hp.current, hp.max
                    );
                } else {
                    info!("{:?} destroyed! Final HP: 0/{}", enemy_entity, hp.max);
                    if user.is_some() {
                        info!("GAME OVER");
                        next_state.set(GameState::GameOver);
                    } else {
                        commands.entity(enemy_entity).despawn();
                    }
                }
            }

            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.2, 0.2, 0.2))),
                MeshMaterial3d(materials.add(Color::srgb(1.0, 1.0, 0.0))),
                Transform::from_translation(turret_pos + Vec3::Y * 2.0),
            ));
        }
    }
}
