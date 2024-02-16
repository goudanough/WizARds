use crate::network::{PlayerHead, PlayerObj};
use crate::speech::RecordingStatus;
use bevy::prelude::*;
use bevy_ggrs::{GgrsSchedule, PlayerInputs};
use bevy_oxr::xr_input::hands::common::HandsResource;
use bevy_oxr::xr_input::hands::HandBone;
use bevy_oxr::xr_input::trackers::OpenXRTracker;
use bevy_xpbd_3d::prelude::*;
pub struct SpellControlPlugin;
use crate::{projectile::*, WizGgrsConfig};

impl Plugin for SpellControlPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ThumbIndexDist { dist: 0.0 })
            .insert_resource(Spell {
                spell_type: SpellType::Red,
                status: SpellStatus::None,
            })
            .insert_resource(SpellCast(0))
            .add_systems(Update, (handle_spell_control, handle_spell_casting))
            .add_systems(GgrsSchedule, spawn_new_spells);
    }
}

#[derive(Copy, Clone)]
pub enum SpellStatus {
    None,
    Prepare,
    Armed,
    Fired,
}

#[derive(Copy, Clone)]
pub enum SpellType {
    Red = 1,
    Blue = 2,
    Green = 3,
}

#[derive(Resource, Copy, Clone)]
pub struct Spell {
    pub spell_type: SpellType,
    pub status: SpellStatus,
}

struct SpellInfo {
    color: Color,
    id: u32,
}

#[derive(Component)]
struct ThumbIndexDistText;

#[derive(Component)]
struct SpellObject;

#[derive(Resource)]
pub struct SpellCast(pub u32);

fn handle_spell_casting(
    mut create_spell: ResMut<Spell>,
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    mut spell_query: Query<(Entity, &mut Transform), (With<SpellObject>, Without<HandBone>)>,

    hands_resource: Res<HandsResource>,
    mut commands: Commands,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spell_cast: ResMut<SpellCast>, //mut clear_color: ResMut<ClearColor>
) {
    let right_hand = hand_bones.get(hands_resource.right.palm).unwrap();

    let dist =
        right_hand.translation - (0.07 * right_hand.rotation.mul_vec3(right_hand.translation));
    let spell_type = create_spell.spell_type;
    let color = match spell_type {
        SpellType::Red => Color::RED,
        SpellType::Blue => Color::BLUE,
        SpellType::Green => Color::GREEN,
    };

    let spell = SpellInfo {
        color,
        id: spell_type as u32,
    };
    match create_spell.status {
        SpellStatus::None => {
            for (entity, _) in spell_query.iter() {
                commands.entity(entity).despawn();
            }
        }
        SpellStatus::Prepare => {
            for (entity, _) in spell_query.iter() {
                commands.entity(entity).despawn();
            }
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::UVSphere {
                        radius: 0.03,
                        ..default()
                    })),
                    material: materials.add(spell.color),
                    transform: Transform::from_xyz(dist.x, dist.y, dist.z),
                    ..default()
                },
                SpellObject,
            ));
            create_spell.status = SpellStatus::Armed;
        }
        SpellStatus::Armed => {
            for (_, mut transform) in spell_query.iter_mut() {
                transform.translation = Transform::from_xyz(dist.x, dist.y, dist.z).translation;
            }
        }
        SpellStatus::Fired => {
            create_spell.status = SpellStatus::None;
            for (entity, _) in spell_query.iter() {
                commands.entity(entity).despawn();
            }

            spell_cast.0 = spell.id;
        }
    }
}

fn spawn_new_spells(
    inputs: Res<PlayerInputs<WizGgrsConfig>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_objs: Query<&PlayerObj, With<PlayerHead>>,
) {
    let mut count: u8 = 0;
    for p in player_objs.iter() {
        let input = inputs[p.handle].0;
        if input.spell != 0 {
            let mesh = meshes.add(Mesh::from(shape::UVSphere {
                radius: 0.03,
                ..default()
            }));

            let material: Handle<StandardMaterial> = if input.spell == 1 {
                materials.add(Color::rgb(1., 0., 0.))
            } else if input.spell == 2 {
                materials.add(Color::rgb(0., 0., 1.))
            } else if input.spell == 3 {
                materials.add(Color::rgb(0., 1., 0.))
            } else {
                materials.add(Color::rgb(1., 1., 1.))
            };

            let collider = Collider::ball(0.03);
            let direction = -input.right_hand_rot.mul_vec3(input.right_hand_pos);
            let transform = Transform {
                translation: input.right_hand_pos + (0.07 * direction),
                ..default()
            };
            let speed = 1.;
            spawn_projectile(
                &mut commands,
                mesh,
                material,
                transform,
                collider,
                direction,
                speed,
                default(),
            );
        }
    }
}

#[derive(Resource)]
pub struct ThumbIndexDist {
    dist: f32,
}

fn handle_spell_control(
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: Res<HandsResource>,
    mut recording_mode: ResMut<RecordingStatus>,
    mut thumb_index_depth_res: ResMut<ThumbIndexDist>,
    mut spell: ResMut<Spell>,
) {
    let thumb_tip_transform = hand_bones.get(hands_resource.left.thumb.tip).unwrap();
    let index_tip_transform = hand_bones.get(hands_resource.left.index.tip).unwrap();
    let middle_tip_transform = hand_bones.get(hands_resource.left.middle.tip).unwrap();

    let thumb_index_dist =
        bevy::math::Vec3::length(thumb_tip_transform.translation - index_tip_transform.translation);
    let thumb_middle_dist = bevy::math::Vec3::length(
        thumb_tip_transform.translation - middle_tip_transform.translation,
    );

    thumb_index_depth_res.dist = thumb_index_dist;
    if thumb_index_dist < 0.01 {
        if !recording_mode.just_started && !recording_mode.recording {
            recording_mode.just_started = true;
            recording_mode.recording = true;
            recording_mode.just_ended = false;
        }
    } else if recording_mode.recording {
        recording_mode.just_ended = true;
    }

    if thumb_middle_dist < 0.01 {
        if let SpellStatus::Armed = spell.status {
            spell.status = SpellStatus::Fired
        }
    }
}
