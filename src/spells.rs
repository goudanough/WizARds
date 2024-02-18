use ::bevy::prelude::*;
use bevy_ggrs::{AddRollbackCommandExtension, GgrsSchedule};
use bevy_oxr::xr_input::hands::common::HandsResource;

use crate::assets::{AssetHandles, MatName, MeshName};
use crate::network::{debug_move_networked_player_objs, PlayerID};
use crate::projectile::{spawn_projectile, update_linear_movement, ProjectileType};
use crate::spell_control::{SelectedSpell, Spell};
use crate::PlayerInput;

pub struct SpellsPlugin;

#[derive(Component)]
pub struct SpellIndicator;

#[derive(Component)]
pub struct TrajectoryIndicator {
    pub despawn_on_fire: bool,
}

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
                .after(debug_move_networked_player_objs),
        );
    }
}

pub fn spawn_spell(commands: &mut Commands, input: PlayerInput, p_id: usize) {
    let spell = input.spell.try_into().unwrap();
    let right_palm_transform = Transform {
        translation: input.right_hand_pos,
        rotation: input.right_hand_rot,
        ..default()
    };
    match spell {
        Spell::Fireball => commands
            .spawn((
                SpellObj,
                FireSpell,
                PlayerID { handle: p_id },
                SpatialBundle {
                    transform: Transform::from_translation(
                        right_palm_transform.translation
                            - 0.1 * Vec3::from(right_palm_transform.local_y()),
                    )
                    .with_rotation(input.right_hand_rot), // TODO currently incorrect direction, needs integrating with a proper aiming system
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
                    transform: Transform::from_translation(
                        right_palm_transform.translation
                            - 0.1 * Vec3::from(right_palm_transform.local_y()),
                    )
                    .with_rotation(input.right_hand_rot), // TODO currently incorrect direction, needs integrating with a proper aiming system
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
    hands_resource: Res<HandsResource>,
    asset_handles: Res<AssetHandles>,
    selected_spell: Res<SelectedSpell>,
) {
    let spell_ind_id = match selected_spell.0.unwrap() {
        Spell::Fireball => commands.spawn((
            SpellIndicator,
            PbrBundle {
                mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                material: asset_handles.mats[MatName::Red as usize].clone(),
                transform: Transform::from_translation(Vec3::new(0.0, -0.1, 0.0))
                    .with_scale(0.2 * Vec3::ONE),
                ..Default::default()
            },
        )),
        Spell::Lightning => commands.spawn((
            SpellIndicator,
            PbrBundle {
                mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
                material: asset_handles.mats[MatName::Blue as usize].clone(),
                transform: Transform::from_translation(Vec3::new(0.0, -0.1, 0.0))
                    .with_scale(0.2 * Vec3::ONE),
                ..Default::default()
            },
        )),
    }
    .id();
    commands
        .get_entity(hands_resource.right.palm)
        .unwrap()
        .push_children(&[spell_ind_id]);
}

pub fn spawn_trajectory_indicator(
    mut commands: Commands,
    hands_resource: Res<HandsResource>,
    asset_handles: Res<AssetHandles>,
    selected_spell: Res<SelectedSpell>,
) {
    let spell_ind_id = match selected_spell.0.unwrap() {
        Spell::Fireball => commands.spawn((
            TrajectoryIndicator {
                despawn_on_fire: true,
            },
            // TODO finish implementing these objects
        )),
        Spell::Lightning => commands.spawn((
            TrajectoryIndicator {
                despawn_on_fire: true,
            },
            // TODO finish implementing these objects
        )),
    }
    .id();
    commands
        .get_entity(hands_resource.right.palm)
        .unwrap()
        .push_children(&[spell_ind_id]);
}
