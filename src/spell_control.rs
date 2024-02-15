use crate::speech::RecordingStatus;
use bevy::prelude::*;
use bevy::render::mesh::shape::Quad;
use bevy_oxr::xr::sys::pfn::QuerySystemTrackedKeyboardFB;
use bevy_oxr::xr_input::hands::common::{
    HandInputDebugRenderer, HandResource, HandsResource, OpenXrHandInput,
};
use bevy_oxr::xr_input::hands::HandBone;
use bevy_oxr::xr_input::trackers::{
    OpenXRController, OpenXRLeftController, OpenXRLeftEye, OpenXRRightController, OpenXRRightEye,
    OpenXRTracker,
};

use bevy_oxr::xr_input::xr_camera::XrCameraBundle;
use bevy_xpbd_3d::prelude::*;
pub struct SpellControlPlugin;
use crate::projectile::*;

impl Plugin for SpellControlPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ThumbIndexDist { dist: 0.0 })
            .insert_resource(Spell {
                spell_type: SpellType::Red,
                status: SpellStatus::None,
            })
            .add_systems(Update, (thumb_index_spell_selection, update_sphere));
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
    Red,
    Blue,
    Green,
}

#[derive(Resource, Copy, Clone)]
pub struct Spell {
    pub spell_type: SpellType,
    pub status: SpellStatus,
}

#[derive(Component)]
struct SpellObject;

fn update_sphere(
    mut create_spell: ResMut<Spell>,
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    mut spell_query: Query<(Entity, &mut Transform), (With<SpellObject>, Without<HandBone>)>,

    hands_resource: Res<HandsResource>,
    mut commands: Commands,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    mut gizmos: Gizmos,
    spatial_query: SpatialQuery,
) {
    let right_hand = hand_bones.get(hands_resource.right.palm).unwrap();
    let right_wrist = hand_bones.get(hands_resource.right.wrist).unwrap();

    let dist =
        right_hand.translation - (0.07 * right_hand.rotation.mul_vec3(right_hand.translation));
    let spell_type = create_spell.spell_type;
    let spell = match spell_type {
        SpellType::Red => Color::RED,
        SpellType::Blue => Color::BLUE,
        SpellType::Green => Color::GREEN,
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
                    material: materials.add(spell),
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

            if let Some(ray_hit) = spatial_query.cast_ray(
                right_wrist.translation,
                -right_hand.rotation.mul_vec3(right_hand.translation),
                1000.0,
                true,
                SpatialQueryFilter::default(),
            ) {
                gizmos.line(
                    dist,
                    right_hand.translation
                        + (-right_hand.rotation.mul_vec3(right_hand.translation))
                            * ray_hit.time_of_impact,
                    Color::RED,
                )
            }
        }
        SpellStatus::Fired => {
            for (entity, _) in spell_query.iter() {
                commands.entity(entity).despawn();
            }

            let mesh = meshes.add(Mesh::from(shape::UVSphere {
                radius: 0.03,
                ..default()
            }));
            let material = materials.add(spell);
            let collider = Collider::ball(0.03);
            let transform = Transform::from_xyz(dist.x, dist.y, dist.z);
            let direction = -right_hand.rotation.mul_vec3(right_hand.translation);
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

            create_spell.status = SpellStatus::None;
        }
    }
}

#[derive(Resource)]
pub struct ThumbIndexDist {
    dist: f32,
}

fn thumb_index_spell_selection(
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

    //println!("{}", thumb_index_dist);
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
        match spell.status {
            SpellStatus::Armed => spell.status = SpellStatus::Fired,
            _ => (),
        }
    }
}
