use bevy::prelude::*;

use crate::logging::{GameEvent, MatchLog};
use crate::user::User;
use crate::GameState;

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
    turret_query: Query<(Entity, &Transform, &Turret)>,
    mut target_query: Query<(Entity, &Transform, &mut Hp, Option<&User>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut match_log: ResMut<MatchLog>,
    mut gizmos: Gizmos,
) {
    let current_time = time.elapsed_secs();

    for (turret_entity, turret_transform, turret) in turret_query.iter() {
        if !turret.is_active(current_time) {
            // Visual indicator for cooldown (e.g., red box on top)
            // We can spawn a temporary marker or change material color.
            // For simplicity, let's draw a red gizmo box above it.
            gizmos.cube(
                Transform::from_translation(turret_transform.translation + Vec3::Y * 2.5)
                    .with_scale(Vec3::splat(0.5)),
                Color::srgb(1.0, 0.0, 0.0),
            );
            continue;
        } else {
            // Visual indicator for ready (green box)
            gizmos.cube(
                Transform::from_translation(turret_transform.translation + Vec3::Y * 2.5)
                    .with_scale(Vec3::splat(0.5)),
                Color::srgb(0.0, 1.0, 0.0),
            );
        }

        let turret_pos = turret_transform.translation;

        let direction_vec = turret.direction.to_vec3();

        let mut closest_target: Option<Entity> = None;
        let mut closest_distance = f32::MAX;

        for (target_entity, target_transform, _, _) in target_query.iter() {
            // Don't shoot owner
            if target_entity == turret.owner {
                continue;
            }

            // Don't shoot other turrets (unless they have HP, which they don't currently)
            // Actually target_query filters for Hp, so structures without Hp are ignored.

            let target_pos = target_transform.translation;
            let to_target = target_pos - turret_pos;
            let distance = to_target.length();

            if distance < 15.0 {
                let target_dir = to_target.normalize();
                let dot = target_dir.dot(direction_vec);

                // 45 degree cone (half angle 22.5) -> cos(22.5) ~= 0.923
                // 90 degree cone (half angle 45) -> cos(45) ~= 0.707
                // User asked for "at least 45 degree cone". Let's assume 45 degrees total width (+/- 22.5).
                // Or maybe 45 degrees to each side (90 total)? "45 degree cone" usually means total angle.
                // Let's go with 0.707 (45 degrees to each side, 90 total) to be generous and make them effective.
                if dot > 0.707 {
                    if distance < closest_distance {
                        closest_distance = distance;
                        closest_target = Some(target_entity);
                    }
                }
            }
        }

        if let Some(target_entity) = closest_target {
            commands.entity(turret_entity).insert(Turret {
                owner: turret.owner,
                direction: turret.direction,
                last_shot: current_time,
            });

            if let Ok((target_entity, target_transform, mut hp, user)) =
                target_query.get_mut(target_entity)
            {
                hp.take_damage(TURRET_DAMAGE);

                // Visual feedback: Line from turret barrel to target
                let barrel_pos = turret_pos + Vec3::Y * 1.5; // Approx barrel height
                gizmos.line(
                    barrel_pos,
                    target_transform.translation,
                    Color::srgb(1.0, 1.0, 0.0),
                );

                match_log.add(GameEvent::DamageDealt {
                    attacker: turret.owner,
                    victim: target_entity,
                    amount: TURRET_DAMAGE,
                    time: current_time,
                });

                if hp.is_alive() {
                    info!(
                        "Turret hit {:?}! HP: {}/{}",
                        target_entity, hp.current, hp.max
                    );
                } else {
                    info!("{:?} destroyed! Final HP: 0/{}", target_entity, hp.max);

                    match_log.add(GameEvent::PlayerEliminated {
                        entity: target_entity,
                        killer: Some(turret.owner),
                        time: current_time,
                    });

                    if user.is_some() {
                        info!("GAME OVER");
                        next_state.set(GameState::GameOver);
                    } else {
                        // Check if it was the Enemy
                        // We can't check With<Enemy> here easily because we are iterating generic targets.
                        // But if it's not User, it's likely the AI.
                        // For now, just despawn.
                        commands.entity(target_entity).despawn();
                    }
                }
            }
        }
    }
}
