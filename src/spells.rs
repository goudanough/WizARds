use std::time::Duration;

use ::bevy::prelude::*;
use bevy::math::primitives;
use bevy_ggrs::{AddRollbackCommandExtension, GgrsSchedule, PlayerInputs, Rollback};
use bevy_hanabi::{ParticleEffect, ParticleEffectBundle};
use bevy_oxr::xr_input::trackers::{OpenXRLeftEye, OpenXRRightEye};
use bevy_xpbd_3d::prelude::*;

use crate::assets::{AssetHandles, EffectName, MatName, MeshName};
use crate::boss::BossHealth;
use crate::network::{move_networked_player_objs, PlayerID, PlayerLeftPalm, PlayerRightPalm};
use crate::projectile::{
    spawn_projectile, update_linear_movement, DamageHit, DamageMask, Projectile,
    ProjectileHitEffect, ProjectileType,
};
use crate::spell_control::{SelectedSpell, Spell, SpellSpawnLocation};
use crate::{PhysLayer, PlayerInput, WizGgrsConfig};
pub struct SpellsPlugin;

#[derive(Component)]
pub struct SpellIndicator;

#[derive(Component)]
pub struct TrajectoryIndicator {
    pub despawn_on_fire: bool,
}

#[derive(Component)]
pub struct StraightLaserTrajInd;

#[derive(Component)]
pub struct SpellObj;

#[derive(Component)]
pub struct MissileSpell;

#[derive(Component)]
pub struct FireSpell;

#[derive(Component)]
pub struct LightningSpell;

#[derive(Component)]
pub struct ParrySpell;

#[derive(Component)]
pub struct ParryObj;

#[derive(Component)]
pub struct ParriedProjectile;

#[derive(Component)]
pub struct ParryTimer(Timer);

#[derive(Component)]
pub struct BombSpell;

#[derive(Component)]
pub struct BombObj;

#[derive(Component)]
pub struct BombExplosionEffect;

#[derive(Component)]
pub struct HandObj;

#[derive(Component)]
pub struct BombTimer(Timer);

#[derive(Component)]
pub struct DespawnTimer(Timer);

#[derive(Component)]
pub struct WallSpell;

// Component for handling the lifetime of a wall.
#[derive(Component)]
struct Wall {
    previous_point: Vec3,
    building: bool,
    timer: Timer,
}
impl Plugin for SpellsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            GgrsSchedule,
            (
                handle_lightning,
                handle_fireballs,
                handle_missiles,
                init_walls,
                handle_walls,
                handle_parry,
                parry_check,
                handle_bomb,
                handle_bomb_explode,
                hand_bomb_collision,
                despawn_timed_entities,
            )
                .chain()
                .before(update_linear_movement)
                .after(move_networked_player_objs),
        )
        .add_systems(
            Update,
            (handle_straight_laser_traj_ind, track_spell_indicator),
        );
    }
}

pub fn spawn_spell(
    commands: &mut Commands,
    input: PlayerInput,
    p_id: usize,
    palm_mid_point: Vec3,
    head_transform: Transform,
) {
    let spell = input.spell.try_into().unwrap();

    match spell {
        Spell::Fireball => commands
            .spawn((
                SpellObj,
                FireSpell,
                PlayerID { handle: p_id },
                SpatialBundle {
                    transform: Transform::from_translation(palm_mid_point)
                        .with_rotation(head_transform.rotation), // TODO currently incorrect direction, needs integrating with a proper aiming system
                    ..Default::default()
                },
            ))
            .add_rollback(),
        Spell::Lightning => commands
            .spawn((
                SpellObj,
                LightningSpell,
                PlayerID { handle: p_id },
                SpatialBundle {
                    transform: Transform::from_translation(palm_mid_point)
                        .with_rotation(head_transform.rotation), // TODO currently incorrect direction, needs integrating with a proper aiming system
                    ..Default::default()
                },
            ))
            .add_rollback(),
        Spell::Parry => commands
            .spawn((
                SpellObj,
                ParrySpell,
                PlayerID { handle: p_id },
                SpatialBundle {
                    transform: Transform::from_translation(palm_mid_point)
                        .with_rotation(head_transform.rotation),
                    ..Default::default()
                },
            ))
            .add_rollback(),
        Spell::Bomb => commands
            .spawn((
                SpellObj,
                BombSpell,
                PlayerID { handle: p_id },
                SpatialBundle {
                    transform: Transform::from_translation(palm_mid_point)
                        .with_rotation(head_transform.rotation), // TODO currently incorrect direction, needs integrating with a proper aiming system
                    ..Default::default()
                },
            ))
            .add_rollback(),
        Spell::Wall => commands
            .spawn((SpellObj, WallSpell, PlayerID { handle: p_id }))
            .add_rollback(),
        Spell::MagicMissile => commands
            .spawn((
                SpellObj,
                MissileSpell,
                PlayerID { handle: p_id },
                SpatialBundle {
                    transform: Transform::from_translation(palm_mid_point)
                        .with_rotation(head_transform.rotation), // TODO currently incorrect direction, needs integrating with a proper aiming system
                    ..Default::default()
                },
            ))
            .add_rollback(),
    };
}

fn handle_fireballs(
    mut commands: Commands,
    spell_objs: Query<(&Transform, Entity), With<FireSpell>>,
    asset_handles: Res<AssetHandles>,
) {
    for (t, e) in spell_objs.iter() {
        spawn_projectile(&mut commands, ProjectileType::Fireball, t, &asset_handles);
        commands.entity(e).despawn_recursive();
    }
}

fn handle_lightning(
    mut commands: Commands,
    spell_objs: Query<(&Transform, Entity), With<LightningSpell>>,
    asset_handles: Res<AssetHandles>,
) {
    for (t, e) in spell_objs.iter() {
        spawn_projectile(
            &mut commands,
            ProjectileType::LightningBolt,
            t,
            &asset_handles,
        );
        commands.entity(e).despawn_recursive();
    }
}

fn handle_bomb(
    mut commands: Commands,
    spell_objs: Query<(&Transform, Entity, &PlayerID), With<BombSpell>>,
    asset_handles: Res<AssetHandles>,
    mut player_left_palms: Query<
        (Entity, &PlayerID),
        (
            With<PlayerLeftPalm>,
            Without<PlayerRightPalm>,
            With<Rollback>,
        ),
    >,
    mut player_right_palms: Query<
        (Entity, &PlayerID),
        (
            Without<PlayerLeftPalm>,
            With<PlayerRightPalm>,
            With<Rollback>,
        ),
    >,
) {
    for (t, e, id) in spell_objs.iter() {
        commands
            .spawn((
                PbrBundle {
                    mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                    material: asset_handles.mats[MatName::Green as usize].clone(),
                    transform: Transform::from_translation(t.translation)
                        .with_scale(0.5 * Vec3::ONE),
                    ..Default::default()
                },
                BombObj,
                PlayerID { handle: id.handle },
                CollisionLayers::new(PhysLayer::Bomb, LayerMask::ALL ^ PhysLayer::BossProjectile),
                BombTimer(Timer::new(Duration::from_secs(5), TimerMode::Once)),
                Collider::sphere(0.1),
            ))
            .add_rollback();

        // To Do: add effects and uncomment this code
        let left_hand_effect = commands
            .spawn((
                // ParticleEffectBundle {
                //     effect: ParticleEffect::new(
                //         asset_handles.effects[EffectName::BombHandEffect as usize].clone(),
                //     ),
                //     ..default()
                // },
                HandObj,
                BombTimer(Timer::new(Duration::from_secs(5), TimerMode::Once)),
            ))
            .add_rollback()
            .id();

        let right_hand_effect = commands
            .spawn((
                // ParticleEffectBundle {
                //     effect: ParticleEffect::new(
                //         asset_handles.effects[EffectName::BombHandEffect as usize].clone(),
                //     ),
                //     ..default()
                // },
                HandObj,
                BombTimer(Timer::new(Duration::from_secs(5), TimerMode::Once)),
            ))
            .add_rollback()
            .id();

        for (left_palm, p) in player_left_palms.iter_mut() {
            if id.handle == p.handle {
                commands
                    .entity(left_hand_effect)
                    .insert(PlayerID { handle: id.handle });
                commands
                    .get_entity(left_palm)
                    .unwrap()
                    .push_children(&[left_hand_effect]);
            }
        }

        for (right_palm, p) in player_right_palms.iter_mut() {
            if id.handle == p.handle {
                commands
                    .entity(right_hand_effect)
                    .insert(PlayerID { handle: id.handle });
                commands
                    .get_entity(right_palm)
                    .unwrap()
                    .push_children(&[right_hand_effect]);
            }
        }

        commands.entity(e).despawn();
    }
}

fn hand_bomb_collision(
    mut commands: Commands,
    collisions: ResMut<Collisions>,
    mut bomb: Query<(Entity, &Transform), With<BombObj>>,
    hands_effect: Query<(Entity, &PlayerID), (With<HandObj>, Without<BombObj>)>,
    player_left_palms: Query<
        (Entity, &Transform, &PlayerID),
        (
            With<PlayerLeftPalm>,
            Without<PlayerRightPalm>,
            With<Rollback>,
        ),
    >,
    player_right_palms: Query<
        (Entity, &Transform, &PlayerID),
        (
            Without<PlayerLeftPalm>,
            With<PlayerRightPalm>,
            With<Rollback>,
        ),
    >,
) {
    for (bomb_entity, bomb_trans) in bomb.iter_mut() {
        for (hand_entity, hand_transform, id) in
            player_left_palms.iter().chain(player_right_palms.iter())
        {
            if collisions.contains(bomb_entity, hand_entity) {
                let hand_bomb_direction =
                    (bomb_trans.translation - hand_transform.translation).normalize();

                commands.entity(bomb_entity).insert(RigidBody::Dynamic);
                commands
                    .entity(bomb_entity)
                    .insert(ExternalForce::new(hand_bomb_direction / 10.0).with_persistence(false));

                for (hand_effect, effect_id) in hands_effect.iter() {
                    if effect_id.handle == id.handle {
                        commands.entity(hand_effect).despawn();
                    }
                }
            }
        }
    }
}

fn handle_bomb_explode(
    mut commands: Commands,
    asset_handles: Res<AssetHandles>,
    time: Res<Time>,
    hands_effect: Query<(Entity, &PlayerID), (With<HandObj>, Without<BombObj>)>,
    mut bomb_objs_query: Query<(Entity, &Transform, &mut BombTimer, &PlayerID), With<BombObj>>,
) {
    for (bomb_e, bomb_trans, mut bomb_timer, id) in bomb_objs_query.iter_mut() {
        if bomb_timer.0.tick(time.delta()).finished() {
            commands.entity(bomb_e).despawn();
            commands
                .spawn((
                    Projectile,
                    ProjectileHitEffect::Damage(DamageHit(DamageMask::FIRE, 25.0)),
                    SpatialBundle {
                        transform: Transform::from_translation(bomb_trans.translation)
                            .with_rotation(bomb_trans.rotation),
                        ..default()
                    },
                    Collider::sphere(1.0),
                    CollisionLayers::new(
                        PhysLayer::PlayerProjectile,
                        (((LayerMask::ALL ^ PhysLayer::Player) ^ PhysLayer::BossProjectile)
                            ^ PhysLayer::Terrain)
                            ^ PhysLayer::PlayerProjectile,
                    ),
                    DespawnTimer(Timer::from_seconds(2.0, TimerMode::Once)),
                ))
                .add_rollback();

            commands.spawn((
                ParticleEffectBundle {
                    effect: ParticleEffect::new(
                        asset_handles.effects[EffectName::BombExplosion as usize].clone(),
                    ),
                    transform: Transform::from_translation(bomb_trans.translation)
                        .with_rotation(bomb_trans.rotation),
                    ..default()
                },
                DespawnTimer(Timer::from_seconds(2.0, TimerMode::Once)),
            ));

            for (hand_effect, effect_id) in hands_effect.iter() {
                if effect_id.handle == id.handle {
                    commands.entity(hand_effect).despawn();
                }
            }
        }
    }
}

fn handle_parry(
    mut commands: Commands,
    left_palms: Query<(Entity, &PlayerID), With<PlayerLeftPalm>>,
    right_palms: Query<(Entity, &PlayerID), With<PlayerRightPalm>>,
    spell_objs: Query<(Entity, &PlayerID), With<ParrySpell>>,
    //asset_handles: Res<AssetHandles>,
) {
    for (e, p) in spell_objs.iter() {
        let parry_left = commands
            .spawn((
                ParryObj,
                PlayerID { handle: p.handle },
                SpatialBundle {
                    ..Default::default()
                },
                // ParticleEffectBundle {
                //     effect: ParticleEffect::new(asset_handles.effects[EffectName::BombExplosion as usize].clone()),
                //     ..default()
                // },
                CollisionLayers::new(
                    PhysLayer::ParryObject,
                    (LayerMask::ALL ^ PhysLayer::Player) ^ PhysLayer::PlayerProjectile,
                ),
                Collider::sphere(0.12),
                ParryTimer(Timer::from_seconds(5.0, TimerMode::Once)),
            ))
            .add_rollback()
            .id();

        let parry_right = commands
            .spawn((
                ParryObj,
                PlayerID { handle: p.handle },
                SpatialBundle {
                    ..Default::default()
                },
                // ParticleEffectBundle {
                //     effect: ParticleEffect::new(asset_handles.effects[EffectName::BombExplosion as usize].clone()),
                //     ..default()
                // },
                CollisionLayers::new(
                    PhysLayer::ParryObject,
                    (LayerMask::ALL ^ PhysLayer::Player) ^ PhysLayer::PlayerProjectile,
                ),
                Collider::sphere(0.12),
                ParryTimer(Timer::from_seconds(5.0, TimerMode::Once)),
            ))
            .add_rollback()
            .id();

        for (left_palm, id) in left_palms.iter() {
            if id.handle == p.handle {
                commands
                    .get_entity(left_palm)
                    .unwrap()
                    .push_children(&[parry_left]);
            }
        }

        for (right_palm, id) in right_palms.iter() {
            if id.handle == p.handle {
                commands
                    .get_entity(right_palm)
                    .unwrap()
                    .push_children(&[parry_right]);
            }
        }

        commands.entity(e).despawn_recursive();
    }
}

fn parry_check(
    mut commands: Commands,
    time: Res<Time>,
    mut parry_objs_query: Query<(Entity, &GlobalTransform, &mut ParryTimer), With<ParryObj>>,
    projectiles: Query<(&Transform, &Handle<StandardMaterial>, &Handle<Mesh>), With<Projectile>>,
    mut collision_event_reader: EventReader<Collision>,
) {
    let collision_events: Vec<Collision> = collision_event_reader.read().cloned().collect();
    for (parry_obj, _, mut parry_timer) in parry_objs_query.iter_mut() {
        if parry_timer.0.tick(time.delta()).finished() {
            commands.entity(parry_obj).despawn();
        }
    }

    for (parry_obj, parry_transform, _) in parry_objs_query.iter_mut() {
        for Collision(contacts) in &collision_events {
            let proj = match parry_obj {
                c if c == contacts.entity1 => contacts.entity2,
                c if c == contacts.entity2 => contacts.entity1,
                _ => continue,
            };

            let Ok((proj_trans, material, mesh)) = projectiles.get(proj) else {
                continue;
            };

            commands.entity(proj).despawn();

            let parry_proj_direction =
                (proj_trans.translation - parry_transform.translation()).normalize();

            commands
                .spawn((
                    ParriedProjectile,
                    ExternalForce::new(parry_proj_direction * 2.0).with_persistence(false),
                    PbrBundle {
                        mesh: mesh.clone(),
                        material: material.clone(),
                        transform: *proj_trans,
                        ..Default::default()
                    },
                    CollisionLayers::new(
                        PhysLayer::PlayerProjectile,
                        ((LayerMask::ALL ^ PhysLayer::Player) ^ PhysLayer::BossProjectile)
                            ^ PhysLayer::ParryObject,
                    ),
                    Collider::sphere(0.1),
                    DespawnTimer(Timer::from_seconds(5.0, TimerMode::Once)),
                    RigidBody::Dynamic,
                ))
                .add_rollback();
            // only parry one thing per frame - quick n dirty way to not spawn two parried guys
            // if we hit one normal projectile with two hands
            return;
        }
    }
}

// Respond to wall spell cast by creating a wall entity.
fn init_walls(
    mut commands: Commands,
    spell_objs: Query<(Entity, &PlayerID), With<WallSpell>>,
    inputs: Res<PlayerInputs<WizGgrsConfig>>,
) {
    for (e, p_id) in spell_objs.iter() {
        let input = inputs[p_id.handle];
        let head_pos = input.0.head_pos;
        commands.spawn((
            SpatialBundle {
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                ..Default::default()
            },
            Wall {
                // Initialise the wall at the players head position.
                previous_point: Vec3::new(head_pos.x, head_pos.y / 2.0, head_pos.z),
                building: true,
                // Initial timer, for wall creation.
                timer: Timer::from_seconds(3.0, TimerMode::Once),
            },
            // PlayerID so we know who's wall it is.
            PlayerID {
                handle: p_id.handle,
            },
        ));
        // Despawn SpellObj,
        commands.entity(e).despawn();
    }
}

// Handle wall creation, and eventual despawning.
fn handle_walls(
    mut commands: Commands,
    mut walls: Query<(&mut Wall, &PlayerID, Entity)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    inputs: Res<PlayerInputs<WizGgrsConfig>>,
    time: Res<Time>,
) {
    for (mut wall, p_id, e) in walls.iter_mut() {
        wall.timer.tick(time.delta());
        // If timer has just finished and we're building, then building is over.
        // Indicate building is over, and start a new timer.
        if wall.timer.just_finished() && wall.building {
            wall.building = false;
            wall.timer = Timer::from_seconds(10.0, TimerMode::Once);
        }
        // If we're building, check if we've moved far enough to spawn a new segment, if we have then spawn a new segment, and update previous position.
        if wall.building {
            let head_pos = inputs[p_id.handle].0.head_pos;
            let head_pos_flat = Vec3::new(head_pos.x, head_pos.y / 2.0, head_pos.z);
            if (head_pos_flat - wall.previous_point).length() >= 0.2 {
                let id = commands
                    .spawn((
                        PbrBundle {
                            mesh: meshes.add(Mesh::from(primitives::Cuboid::new(
                                (head_pos_flat - wall.previous_point).length(),
                                head_pos.y,
                                0.1,
                            ))),
                            material: materials.add(Color::WHITE),
                            // Aim is that the translation is the centre of the wall, and it's faced perpendicular to the "line" of the wall.
                            transform: Transform::from_translation(
                                (head_pos_flat + wall.previous_point) / 2.0,
                            )
                            .looking_to(
                                (head_pos_flat - wall.previous_point).cross(Vec3::Y),
                                Vec3::Y,
                            ),
                            ..default()
                        },
                        CollisionLayers::new(
                            PhysLayer::Terrain,
                            (LayerMask::ALL ^ PhysLayer::Player) ^ PhysLayer::Terrain,
                        ),
                        Collider::cuboid(
                            (head_pos_flat - wall.previous_point).length(),
                            head_pos.y,
                            0.1,
                        ),
                    ))
                    .add_rollback()
                    .id();
                commands.entity(e).add_child(id);
                wall.previous_point = head_pos_flat;
            }
        // If we're not building, and the timer has finished, then despawn the wall.
        } else if wall.timer.just_finished() {
            commands.entity(e).despawn_recursive();
        }
    }
}

// Handle cast missile spells.
fn handle_missiles(
    mut commands: Commands,
    spell_objs: Query<(&Transform, Entity), With<MissileSpell>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut boss_health: Query<&mut BossHealth>,
    left_eye: Query<&Transform, (With<OpenXRLeftEye>, Without<StraightLaserTrajInd>)>,
    right_eye: Query<&Transform, (With<OpenXRRightEye>, Without<StraightLaserTrajInd>)>,
    spatial_query: SpatialQuery,
) {
    let left_eye = left_eye.get_single().unwrap();
    let right_eye = right_eye.get_single().unwrap();

    let head_transform =
        Transform::from_translation(left_eye.translation.lerp(right_eye.translation, 0.5))
            .with_rotation(left_eye.rotation);

    for (t, e) in spell_objs.iter() {
        // Spell is hitscan, so raycast to find what the spell hits.
        let mut beam_length = 50.0;
        if let Some(target) = spatial_query.cast_ray(
            t.translation,
            head_transform.forward(),
            beam_length,
            true,
            SpatialQueryFilter::from_mask([PhysLayer::Terrain, PhysLayer::Boss]),
        ) {
            // If we've hit the boss, damage it.
            if let Ok(mut health) = boss_health.get_mut(target.entity) {
                // TODO change this to use the damage type we actually want to use for this.
                if health.damage_mask.intersect(&DamageMask::FIRE) {
                    health.current -= 25.0;
                }
            }
            beam_length = target.time_of_impact;
        };
        // Despawn SpellObj, since the spell has been handled now.
        commands.entity(e).despawn();

        // If the spell hits anything, spawn a visual to represent this.
        let beam_start = t.translation;
        let mesh = meshes.add(Cylinder::new(0.01, beam_length));

        commands
            .spawn((
                PbrBundle {
                    mesh,
                    material: mats.add(Color::WHITE),
                    transform: Transform::from_translation(
                        beam_start + (0.5 * beam_length * Vec3::from(t.forward())),
                    )
                    .looking_to(t.up().into(), t.forward().into()),
                    ..default()
                },
                // This visual should despawn eventually.
                DespawnTimer(Timer::from_seconds(0.2, TimerMode::Once)),
            ))
            .add_rollback();
    }
}

pub fn spawn_spell_indicator(
    mut commands: Commands,
    asset_handles: Res<AssetHandles>,
    selected_spell: Res<SelectedSpell>,
    palm_mid_point: Res<SpellSpawnLocation>,
) {
    match selected_spell.0.unwrap() {
        Spell::Fireball => commands.spawn((
            SpellIndicator,
            PbrBundle {
                mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                material: asset_handles.mats[MatName::Red as usize].clone(),
                transform: Transform::from_translation(palm_mid_point.0)
                    .with_scale(0.2 * Vec3::ONE),
                ..Default::default()
            },
        )),
        Spell::Lightning => commands.spawn((
            SpellIndicator,
            PbrBundle {
                mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                material: asset_handles.mats[MatName::Blue as usize].clone(),
                transform: Transform::from_translation(palm_mid_point.0)
                    .with_scale(0.2 * Vec3::ONE),
                ..Default::default()
            },
        )),
        Spell::Parry => commands.spawn((
            SpellIndicator,
            PbrBundle {
                mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                material: asset_handles.mats[MatName::Purple as usize].clone(),
                transform: Transform::from_translation(palm_mid_point.0)
                    .with_scale(0.2 * Vec3::ONE),
                ..Default::default()
            },
        )),
        Spell::Bomb => commands.spawn((
            SpellIndicator,
            PbrBundle {
                mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                material: asset_handles.mats[MatName::Green as usize].clone(),
                transform: Transform::from_translation(palm_mid_point.0)
                    .with_scale(0.2 * Vec3::ONE),
                ..Default::default()
            },
        )),
        Spell::Wall => commands.spawn((
            SpellIndicator,
            PbrBundle {
                mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                material: asset_handles.mats[MatName::Blue as usize].clone(),
                transform: Transform::from_translation(palm_mid_point.0)
                    .with_scale(0.2 * Vec3::ONE),
                ..Default::default()
            },
        )),
        Spell::MagicMissile => commands.spawn((
            SpellIndicator,
            PbrBundle {
                mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                material: asset_handles.mats[MatName::Red as usize].clone(),
                transform: Transform::from_translation(palm_mid_point.0)
                    .with_scale(0.2 * Vec3::ONE),
                ..Default::default()
            },
        )),
    };
}

pub fn spawn_trajectory_indicator(
    mut commands: Commands,
    selected_spell: Res<SelectedSpell>,
    palm_mid_point: Res<SpellSpawnLocation>,
) {
    match selected_spell.0.unwrap() {
        Spell::Fireball => {
            commands.spawn((
                TrajectoryIndicator {
                    despawn_on_fire: true,
                },
                StraightLaserTrajInd,
                SpatialBundle::default(),
            ));
        }
        Spell::Lightning => {
            commands.spawn((
                TrajectoryIndicator {
                    despawn_on_fire: true,
                },
                StraightLaserTrajInd,
                SpatialBundle {
                    transform: Transform::from_translation(palm_mid_point.0),
                    ..default()
                },
            ));
        }
        Spell::Parry => {}
        Spell::Bomb => {}
        Spell::Wall => {}
        Spell::MagicMissile => {
            commands.spawn((
                TrajectoryIndicator {
                    despawn_on_fire: true,
                },
                StraightLaserTrajInd,
                SpatialBundle {
                    transform: Transform::from_translation(palm_mid_point.0),
                    ..default()
                },
            ));
        }
    }
}

fn handle_straight_laser_traj_ind(
    mut traj_ind: Query<&mut Transform, With<StraightLaserTrajInd>>,
    spatial_query: SpatialQuery,
    mut gizmos: Gizmos,
    left_eye: Query<&Transform, (With<OpenXRLeftEye>, Without<StraightLaserTrajInd>)>,
    right_eye: Query<&Transform, (With<OpenXRRightEye>, Without<StraightLaserTrajInd>)>,
    palm_mid_point: Res<SpellSpawnLocation>,
) {
    let left_eye = left_eye.get_single().unwrap();
    let right_eye = right_eye.get_single().unwrap();

    let head_transform =
        Transform::from_translation(left_eye.translation.lerp(right_eye.translation, 0.5))
            .with_rotation(left_eye.rotation);

    let mut t = match traj_ind.get_single_mut() {
        Ok(t) => t,
        _ => return,
    };

    t.translation = palm_mid_point.0;

    let max_travel = 50.0;

    let ray_travel = match spatial_query.cast_ray(
        t.translation,
        head_transform.forward(),
        max_travel,
        true,
        SpatialQueryFilter::from_mask([PhysLayer::Terrain, PhysLayer::Boss]),
    ) {
        Some(ray_hit) => ray_hit.time_of_impact,
        None => max_travel,
    };
    gizmos.line(
        t.translation,
        t.translation + (head_transform.forward() * ray_travel),
        Color::RED,
    ); // TODO don't use gizmos for line drawing
}

fn track_spell_indicator(
    palm_mid_point: Res<SpellSpawnLocation>,
    mut spell_indicator: Query<&mut Transform, With<SpellIndicator>>,
) {
    let mut t = match spell_indicator.get_single_mut() {
        Ok(t) => t,
        _ => return,
    };

    t.translation = palm_mid_point.0;
}

// Generic system for despawning entities on a timer.
fn despawn_timed_entities(
    mut commands: Commands,
    time: Res<Time>,
    mut objects: Query<(Entity, &mut DespawnTimer)>,
) {
    for (entity, mut timer) in objects.iter_mut() {
        if timer.0.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn();
        }
    }
}
