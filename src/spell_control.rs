use bevy::prelude::*;
use bevy_ggrs::{GgrsSchedule, PlayerInputs};
use bevy_hanabi::{ParticleEffect, ParticleEffectBundle};
use bevy_oxr::xr_input::hands::HandBone;
use bevy_oxr::xr_input::trackers::{OpenXRRightEye, OpenXRTracker};
use bevy_oxr::xr_input::{hands::common::HandsResource, trackers::OpenXRLeftEye};

use crate::assets::EffectName;
use crate::spells::DespawnTimer;
use crate::{
    assets::{AssetHandles, MatName, MeshName},
    network::{LocalPlayerID, PlayerHead, PlayerID},
    speech::{
        check_fingers_close, fetch_recogniser, RecognizedWord, RecordingStatus, SpeechRecognizer,
    },
    spells::{
        spawn_spell, spawn_spell_indicator, spawn_trajectory_indicator, SpellIndicator, SpellObj,
        TrajectoryIndicator,
    },
    WizGgrsConfig,
};

pub struct SpellControlPlugin;

#[derive(Copy, Clone)]
pub enum Spell {
    // don't use 0! it's used to represent no spell in the player inputs
    Fireball = 1,
    Lightning = 2,
    Parry = 3,
    Bomb = 4,
    Wall = 5,
    MagicMissile = 6,
}

#[derive(Debug)]
pub struct SpellConvError;
impl TryFrom<u32> for Spell {
    type Error = SpellConvError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Spell::Fireball),
            2 => Ok(Spell::Lightning),
            3 => Ok(Spell::Parry),
            4 => Ok(Spell::Bomb),
            5 => Ok(Spell::Wall),
            6 => Ok(Spell::MagicMissile),
            _ => Err(SpellConvError),
        }
    }
}

#[derive(States, Debug, Hash, Eq, PartialEq, Clone, Default)]
pub enum SpellStatus {
    #[default]
    None,
    OnCooldown,
    Armed,
    Fire,
}
#[derive(Component)]
pub struct CooldownIndicator;

#[derive(Resource)]
pub struct SpellSpawnLocation(pub Vec3);

#[derive(Resource)]
pub struct SelectedSpell(pub Option<Spell>);

#[derive(Resource, Clone)]
pub struct QueuedSpell(pub Option<Spell>);

#[derive(Resource)]
pub struct SpellCooldown(Timer);

const SPELL_GRAMMAR: [&str; 6] = ["fireball", "lightning", "wind", "fire", "earth", "ice"];

impl Plugin for SpellControlPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SpellStatus>()
            .insert_resource(SpeechRecognizer(fetch_recogniser(&SPELL_GRAMMAR)))
            .insert_resource(SelectedSpell(None))
            .insert_resource(QueuedSpell(None))
            .insert_resource(SpellCooldown(Timer::from_seconds(
                5.0,
                TimerMode::Repeating,
            )))
            .add_systems(
                OnEnter(RecordingStatus::Success),
                (
                    select_spell.run_if(in_state(SpellStatus::None)),
                    attempted_spell_on_cooldown.run_if(in_state(SpellStatus::Armed)),
                ),
            )
            .insert_resource(SpellSpawnLocation(Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }))
            .add_systems(
                Update,
                palm_mid_point_track
                    .run_if(in_state(SpellStatus::None).or_else(in_state(SpellStatus::Armed))),
            )
            .add_systems(
                Update,
                check_spell_fire_input.run_if(in_state(SpellStatus::Armed)),
            )
            .add_systems(
                Update,
                check_if_done_firing.run_if(in_state(SpellStatus::Fire)),
            )
            .add_systems(
                Update,
                (handle_cooldown_timer, track_cooldown_indicator)
                    .run_if(in_state(SpellStatus::OnCooldown)),
            )
            .add_systems(OnEnter(SpellStatus::Armed), spawn_spell_indicator)
            .add_systems(OnEnter(SpellStatus::Armed), spawn_trajectory_indicator)
            .add_systems(OnExit(SpellStatus::Armed), despawn_spell_indicator)
            .add_systems(OnExit(SpellStatus::Armed), despawn_trajectory_indictaor)
            .add_systems(OnEnter(SpellStatus::Fire), queue_new_spell)
            .add_systems(OnExit(SpellStatus::Fire), despawn_trajectory_indictaor)
            .add_systems(OnEnter(SpellStatus::OnCooldown), spawn_cooldown_indicator)
            .add_systems(GgrsSchedule, spawn_new_spell_entities);
    }
}

fn check_spell_fire_input(
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: Res<HandsResource>,
    mut next_spell_state: ResMut<NextState<SpellStatus>>,
) {
    if !check_fingers_close(hand_bones, &hands_resource) {
        next_spell_state.set(SpellStatus::Fire)
    }
}

fn despawn_spell_indicator(mut commands: Commands, spell_ind: Query<Entity, With<SpellIndicator>>) {
    if let Ok(indicator) = spell_ind.get_single() {
        commands.entity(indicator).despawn_recursive();
    }
}

fn queue_new_spell(mut spell_queue: ResMut<QueuedSpell>, selected_spell: Res<SelectedSpell>) {
    spell_queue.0 = selected_spell.0;
}

fn despawn_trajectory_indictaor(
    mut commands: Commands,
    traj_ind: Query<(Entity, &TrajectoryIndicator)>,
    spell_state: Res<State<SpellStatus>>,
) {
    if let Ok((indicator_e, indicator_comp)) = traj_ind.get_single() {
        if *spell_state.get() == SpellStatus::Fire && !indicator_comp.despawn_on_fire {
            return;
        }
        commands.entity(indicator_e).despawn_recursive();
    }
}

fn check_if_done_firing(
    spell_obj: Query<(Entity, &PlayerID), With<SpellObj>>,
    mut next_spell_state: ResMut<NextState<SpellStatus>>,
    local_p_id: Res<LocalPlayerID>,
) {
    if spell_obj
        .iter()
        .filter(|(_, p_id)| p_id.handle == local_p_id.handle)
        .count()
        == 0
    {
        next_spell_state.set(SpellStatus::OnCooldown);
    }
}

fn spawn_new_spell_entities(
    inputs: Res<PlayerInputs<WizGgrsConfig>>,
    mut commands: Commands,
    player_objs: Query<&PlayerID, With<PlayerHead>>,
    spawn_location: Res<SpellSpawnLocation>,
) {
    for p in player_objs.iter() {
        let input = inputs[p.handle].0;

        let head_transform = Transform::from_translation(input.head_pos.lerp(input.head_pos, 0.5))
            .with_rotation(input.head_rot);

        if input.spell != 0 {
            spawn_spell(
                &mut commands,
                input,
                p.handle,
                spawn_location.0,
                head_transform,
            );
        }
    }
}

fn select_spell(
    word: Res<RecognizedWord>,
    mut next_spell_state: ResMut<NextState<SpellStatus>>,
    mut selected_spell: ResMut<SelectedSpell>,
) {
    let (next_s, s_spell) = match &word.0[..] {
        "fireball" => (SpellStatus::Armed, Some(Spell::Fireball)),
        "lightning" => (SpellStatus::Armed, Some(Spell::Lightning)),
        "wind" => (SpellStatus::Armed, Some(Spell::Parry)),
        "fire" => (SpellStatus::Armed, Some(Spell::Bomb)),
        "earth" => (SpellStatus::Armed, Some(Spell::Wall)),
        "ice" => (SpellStatus::Armed, Some(Spell::MagicMissile)),
        _ => (SpellStatus::None, None),
    };

    next_spell_state.set(next_s);
    selected_spell.0 = s_spell;
}

fn palm_mid_point_track(
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: Res<HandsResource>,
    mut palms_mid_point_res: ResMut<SpellSpawnLocation>,
) {
    let left_palm = hand_bones
        .get(hands_resource.left.palm)
        .unwrap()
        .translation;

    let right_palm = hand_bones
        .get(hands_resource.right.palm)
        .unwrap()
        .translation;

    palms_mid_point_res.0 = left_palm.lerp(right_palm, 0.5);
}

fn spawn_cooldown_indicator(mut commands: Commands, asset_handles: Res<AssetHandles>) {
    commands.spawn((
        PbrBundle {
            mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
            material: asset_handles.mats[MatName::Red as usize].clone(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0))
                .with_scale(0.2 * Vec3::ONE),
            ..Default::default()
        },
        CooldownIndicator,
    ));
}

fn track_cooldown_indicator(
    left_eye: Query<&Transform, (With<OpenXRLeftEye>,)>,
    right_eye: Query<&Transform, (With<OpenXRRightEye>,)>,
    mut cooldown_indicator: Query<&mut Transform, With<CooldownIndicator>>,
) {
    let left_eye = left_eye.get_single().unwrap();
    let right_eye = right_eye.get_single().unwrap();

    let head_pos = left_eye.translation.lerp(right_eye.translation, 0.5);
    let head_rot = left_eye.rotation;

    for mut cooldown_transform in cooldown_indicator.iter_mut() {
        let yaw = head_rot.to_euler(EulerRot::XYZ).2;
        cooldown_transform.translation = Transform::from_xyz(
            head_pos.x - yaw.sin() * 0.5,
            head_pos.y / 2.0,
            head_pos.z - yaw.cos() * 0.5,
        )
        .translation;

        cooldown_transform.rotation = Quat::from_euler(EulerRot::XYZ, 0.0, yaw, 0.0);
    }
}

fn handle_cooldown_timer(
    mut commands: Commands,
    time: Res<Time>,
    cooldown_indicator: Query<Entity, With<CooldownIndicator>>,
    mut next_spell_state: ResMut<NextState<SpellStatus>>,
    mut cooldown: ResMut<SpellCooldown>,
) {
    if cooldown.0.tick(time.delta()).just_finished() {
        for e in cooldown_indicator.iter() {
            commands.entity(e).despawn();
        }
        next_spell_state.set(SpellStatus::None);
    }
}

fn attempted_spell_on_cooldown(
    mut commands: Commands,
    //spawn_location: Res<SpellSpawnLocation>,
    //asset_handles: Res<AssetHandles>,
) {
    commands.spawn((
        // ParticleEffectBundle {
        //     effect: ParticleEffect::new(
        //         asset_handles.effects[EffectName::CooldownFizzle as usize].clone(),
        //     ),
        //     transform: Transform::from_translation(spawn_location.0),
        //     ..default()
        // },
        DespawnTimer(Timer::from_seconds(2.0, TimerMode::Once)),
    ));
}
