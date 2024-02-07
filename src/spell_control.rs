use bevy::prelude::*;
use bevy_oxr::xr_input::trackers::{
    OpenXRController, OpenXRLeftController, OpenXRRightController, OpenXRTracker,OpenXRLeftEye, OpenXRRightEye};
use bevy_oxr::xr_input::hands::HandBone;
use bevy_oxr::xr_input::hands::common::{HandInputDebugRenderer, OpenXrHandInput, HandResource, HandsResource};

use crate::speech::RecordingStatus;
pub struct SpellControlPlugin;


impl Plugin for SpellControlPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(ThumbIndexDist {dist:0.0})
        .insert_resource(Spell{spell_type: SpellType::Red, status: SpellStatus::None})
        .add_systems(Startup, spawn_text)
        .add_systems(Update, (thumb_index_spell_selection, update_sphere, update_thumb_index_depth_text));
    }
}


#[derive(Copy, Clone)]
pub enum SpellStatus {
    None,
    Prepare,
    Armed,
    Fired
}

#[derive(Copy, Clone)]
pub enum SpellType{
    Red,
    Blue,
    Green
}

#[derive(Resource, Copy, Clone)]
pub struct Spell {
    pub spell_type: SpellType,
    pub status: SpellStatus
}


#[derive(Component)]
struct ThumbIndexDistText;

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
    //mut clear_color: ResMut<ClearColor>
) {
    let right_hand = hand_bones.get(hands_resource.right.palm).unwrap();

    let dist = right_hand.translation - (0.07* right_hand.rotation.mul_vec3(right_hand.translation));
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
        },
        SpellStatus::Prepare => {  
            for (entity, _) in spell_query.iter() {
                commands.entity(entity).despawn();
                }
            commands.spawn((PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere{radius:0.03, ..default()})),
                material: materials.add(spell.into()),
                transform: Transform::from_xyz(dist.x, dist.y, dist.z),
                ..default()
            }, SpellObject));
            create_spell.status = SpellStatus::Armed;
        },
        SpellStatus::Armed => {
            for (_, mut transform) in spell_query.iter_mut() {
                transform.translation = Transform::from_xyz(dist.x, dist.y, dist.z).translation;
            }
        },
        SpellStatus::Fired => {
            for (entity, _) in spell_query.iter() {
                commands.entity(entity).despawn();
            }
            todo!("Add Spell firing mechanic");
        }
    }   
}


#[derive(Resource)]
pub struct ThumbIndexDist {
    dist: f32
}

fn thumb_index_spell_selection(
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: Res<HandsResource>,
    mut recording_mode: ResMut<RecordingStatus>,
    mut thumb_index_depth_res: ResMut<ThumbIndexDist>,
    mut spell: ResMut<Spell>
) {
    let thumb_tip_transform = hand_bones.get(hands_resource.left.thumb.tip).unwrap();
    let index_tip_transform = hand_bones.get(hands_resource.left.index.tip).unwrap();
    let middle_tip_transform = hand_bones.get(hands_resource.left.middle.tip).unwrap();
    
    let thumb_index_dist = bevy::math::Vec3::length(thumb_tip_transform.translation - index_tip_transform.translation);
    let thumb_middle_dist = bevy::math::Vec3::length(thumb_tip_transform.translation - middle_tip_transform.translation);

    println!("{}", thumb_index_dist);
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
            _ => ()
        }
    }

}

fn spawn_text(mut commands: Commands) {
    commands.spawn((TextBundle::from_sections([
        TextSection::new(
        "Dist Thumb to Index: ",
        TextStyle {
            font_size: 100.0,
            color: Color::RED,
            ..default()
        }),
        TextSection::from_style(
            TextStyle {
                font_size: 60.0,
                color: Color::GOLD,
                ..default()
            })
    ]), 
ThumbIndexDistText));
}

fn update_thumb_index_depth_text(
    mut query: Query<&mut Text, With<ThumbIndexDistText>>,
    thumb_index_dist: Res<ThumbIndexDist>
) {
    for mut text in &mut query {
        text.sections[1].value = thumb_index_dist.dist.to_string();
    }
}


/* 
fn hand_location(
    mut commands: Commands,
    left_eye: Query<&Transform, With<OpenXRLeftEye>>,
    right_eye: Query<&Transform, With<OpenXRRightEye>>,
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: Res<HandsResource>,
    mut recording_mode: ResMut<RecordingStatus>,
) {
    let left_eye = left_eye.get_single().unwrap();
    let right_eye = right_eye.get_single().unwrap();
    let left_hand = hand_bones.get(hands_resource.left.palm).unwrap();
    let right_hand = hand_bones.get(hands_resource.right.palm).unwrap();
    //let player = local_player.0.first().unwrap();

    let head_pos = left_eye.translation.lerp(right_eye.translation, 0.5);

    let left_hand_head_dist = bevy::math::Vec3::length(head_pos - left_hand.translation);

    if left_hand_head_dist < 0.4 {
        if !recording_mode.just_started && !recording_mode.recording {
            recording_mode.just_started = true;
            recording_mode.recording = true;
            recording_mode.just_ended = false;
        }
    } else if recording_mode.recording {
        recording_mode.just_ended = true;
        
    }
}
*/
