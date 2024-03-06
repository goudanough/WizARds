use bevy::prelude::*;
use bevy_ggrs::{GgrsSchedule, PlayerInputs};
use bevy_oxr::xr_input::hands::common::HandsResource;
use bevy_oxr::xr_input::hands::HandBone;
use bevy_oxr::xr_input::trackers::OpenXRTracker;

use crate::{
    network::{PlayerHead, PlayerID, LOCAL_PLAYER_HNDL},
    speech::{fetch_recogniser, get_recognized_words, RecordingStatus, VoiceClip},
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
}

#[derive(Resource)]
pub struct SpellRecognizer(vosk::Recognizer);

#[derive(Debug)]
pub struct SpellConvError;
impl TryFrom<u32> for Spell {
    type Error = SpellConvError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Spell::Fireball),
            2 => Ok(Spell::Lightning),
            _ => Err(SpellConvError),
        }
    }
}

#[derive(States, Debug, Hash, Eq, PartialEq, Clone, Default)]
pub enum SpellStatus {
    #[default]
    None,
    Armed,
    Fire,
}

#[derive(Resource)]
pub struct SelectedSpell(pub Option<Spell>);

#[derive(Resource, Clone)]
pub struct QueuedSpell(pub Option<Spell>);

const SPELL_GRAMMAR: [&str; 2] = ["fireball", "lightning"];

impl Plugin for SpellControlPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SpellStatus>()
            .insert_resource(SpellRecognizer(fetch_recogniser(&SPELL_GRAMMAR)))
            .insert_resource(SelectedSpell(None))
            .insert_resource(QueuedSpell(None))
            .add_systems(OnExit(RecordingStatus::Recording), select_spell)
            .add_systems(
                Update,
                check_spell_fire_input.run_if(in_state(SpellStatus::Armed)),
            )
            .add_systems(
                Update,
                check_if_done_firing.run_if(in_state(SpellStatus::Fire)),
            )
            .add_systems(OnEnter(SpellStatus::Armed), spawn_spell_indicator)
            .add_systems(OnEnter(SpellStatus::Armed), spawn_trajectory_indicator)
            .add_systems(OnExit(SpellStatus::Armed), despawn_spell_indicator)
            .add_systems(OnExit(SpellStatus::Armed), despawn_trajectory_indictaor)
            .add_systems(OnEnter(SpellStatus::Fire), queue_new_spell)
            .add_systems(OnExit(SpellStatus::Fire), despawn_trajectory_indictaor)
            .add_systems(GgrsSchedule, spawn_new_spell_entities);
    }
}

fn check_spell_fire_input(
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: Res<HandsResource>,
    mut next_spell_state: ResMut<NextState<SpellStatus>>,
) {
    let thumb_tip = hand_bones.get(hands_resource.left.thumb.tip).unwrap();
    let middle_tip = hand_bones.get(hands_resource.left.middle.tip).unwrap();
    let thumb_middle_dist = (thumb_tip.translation - middle_tip.translation).length();

    if thumb_middle_dist < 0.02 {
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
) {
    if !spell_obj
        .iter()
        .any(|(_, p_id)| p_id.handle == LOCAL_PLAYER_HNDL)
    {
        next_spell_state.set(SpellStatus::None);
    }
}

fn spawn_new_spell_entities(
    inputs: Res<PlayerInputs<WizGgrsConfig>>,
    mut commands: Commands,
    player_objs: Query<&PlayerID, With<PlayerHead>>,
) {
    for p in player_objs.iter() {
        let input = inputs[p.handle].0;
        if input.spell != 0 {
            spawn_spell(&mut commands, input, p.handle);
        }
    }
}

fn select_spell(
    clip: Res<VoiceClip>,
    mut recogniser: ResMut<SpellRecognizer>,
    mut next_spell_state: ResMut<NextState<SpellStatus>>,
    mut selected_spell: ResMut<SelectedSpell>,
) {
    let words = get_recognized_words(&clip, &mut recogniser.0);
    let last_word = words.last().unwrap_or("");

    let (next_s, s_spell) = match last_word {
        "fireball" => (SpellStatus::Armed, Some(Spell::Fireball)),
        "lightning" => (SpellStatus::Armed, Some(Spell::Lightning)),
        _ => (SpellStatus::None, None),
    };
    next_spell_state.set(next_s);
    selected_spell.0 = s_spell;
}
