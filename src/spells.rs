use ::bevy::prelude::*;
use bevy_ggrs::{AddRollbackCommandExtension, GgrsSchedule};
use bevy_oxr::xr_input::trackers::{OpenXRLeftEye, OpenXRRightEye};
use bevy_xpbd_3d::plugins::spatial_query::{SpatialQuery, SpatialQueryFilter};

use crate::assets::{AssetHandles, MatName, MeshName};
use crate::network::{move_networked_player_objs, PlayerID};
use crate::projectile::{spawn_projectile, update_linear_movement, ProjectileType};
use crate::spell_control::{SelectedSpell, Spell, SpellSpawnLocation};
use crate::{PhysLayer, PlayerInput};

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
pub struct FireSpell;

#[derive(Component)]
pub struct LightningSpell;

impl Plugin for SpellsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            GgrsSchedule,
            (handle_lightning, handle_fireballs)
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
        head_transform.translation + (head_transform.forward() * ray_travel),
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
