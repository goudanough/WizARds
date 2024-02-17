use crate::{
    assets::{AssetHandles, MatName, MeshName},
    network::{LocalPlayerID, PlayerHead, PlayerID},
    speech::{collect_voice, recognise_voice, start_voice},
};
use bevy::prelude::*;
use bevy_ggrs::{GgrsSchedule, PlayerInputs};
use bevy_oxr::xr_input::hands::common::HandsResource;
use bevy_oxr::xr_input::hands::HandBone;
use bevy_oxr::xr_input::trackers::OpenXRTracker;
pub struct SpellControlPlugin;
use crate::{projectile::*, WizGgrsConfig};

#[derive(Copy, Clone)]
pub enum Spell {
    Fireball = 1,
    Lightning = 2,
}

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
    VoiceRecording,
    Determine,
    Armed,
    Fire,
}

#[derive(Resource)]
pub struct SelectedSpell(pub Option<Spell>);

#[derive(Resource, Clone)]
pub struct QueuedSpell(pub Option<Spell>);

impl Into<u32> for QueuedSpell {
    fn into(self) -> u32 {
        match self.0 {
            Some(s) => s as u32,
            None => 0,
        }
    }
}

impl Plugin for SpellControlPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SpellStatus>()
            .insert_resource(SelectedSpell(None))
            .insert_resource(QueuedSpell(None))
            // .add_systems(Update, select_spell.run_if(in_state(SpellStatus::None)))
            // .add_systems(OnEnter(SpellStatus::Armed(())), create_spell_indicator)
            // .add_systems(Update, cast_spell.run_if(in_state(SpellStatus::Armed(()))))
            // .add_systems(OnEnter(SpellStatus::None(())), fire_spell)
            // .add_systems(Update, (handle_spell_control, handle_spell_casting))
            .add_systems(
                Update,
                collect_voice.run_if(in_state(SpellStatus::VoiceRecording)),
            )
            .add_systems(OnEnter(SpellStatus::VoiceRecording), start_voice)
            .add_systems(OnEnter(SpellStatus::Determine), recognise_voice)
            .add_systems(
                Update,
                check_spell_select_input.run_if(
                    in_state(SpellStatus::None)
                        .or_else(in_state(SpellStatus::VoiceRecording))
                        .or_else(in_state(SpellStatus::Armed)),
                ),
            )
            .add_systems(OnEnter(SpellStatus::Armed), spawn_spell_indicator)
            .add_systems(
                Update,
                check_spell_fire_input.run_if(in_state(SpellStatus::Armed)),
            )
            .add_systems(OnExit(SpellStatus::Armed), despawn_spell_indicator)
            .add_systems(OnEnter(SpellStatus::Fire), queue_new_spell)
            .add_systems(
                Update,
                check_if_done_firing.run_if(in_state(SpellStatus::Fire)),
            )
            .add_systems(GgrsSchedule, spawn_new_spell_entities);
    }
}

fn check_spell_select_input(
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: Res<HandsResource>,
    spell_state: Res<State<SpellStatus>>,
    mut next_spell_state: ResMut<NextState<SpellStatus>>,
) {
    let thumb_tip = hand_bones.get(hands_resource.left.thumb.tip).unwrap();
    let index_tip = hand_bones.get(hands_resource.left.index.tip).unwrap();
    let thumb_index_dist = (thumb_tip.translation - index_tip.translation).length();

    if let SpellStatus::VoiceRecording = spell_state.get() {
        if thumb_index_dist > 0.01 {
            next_spell_state.set(SpellStatus::Determine);
        }
    } else if thumb_index_dist < 0.01 {
        next_spell_state.set(SpellStatus::VoiceRecording);
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

    if thumb_middle_dist < 0.01 {
        next_spell_state.set(SpellStatus::Fire)
    }
}

fn spawn_spell_indicator(
    mut commands: Commands,
    hands_resource: Res<HandsResource>,
    asset_handles: Res<AssetHandles>,
    selected_spell: Res<SelectedSpell>,
) {
    let spell_ind_id = match selected_spell.0.unwrap() {
        Spell::Fireball => commands.spawn(PbrBundle {
            mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
            material: asset_handles.mats[MatName::Red as usize].clone(),
            transform: Transform::from_translation(Vec3::new(0.5, 0.0, 0.0))
                .with_scale(0.2 * Vec3::ONE),
            ..Default::default()
        }),
        Spell::Lightning => commands.spawn(PbrBundle {
            mesh: asset_handles.meshes[MeshName::Sphere as usize].clone(),
            material: asset_handles.mats[MatName::Blue as usize].clone(),
            transform: Transform::from_translation(Vec3::new(0.5, 0.0, 0.0))
                .with_scale(0.2 * Vec3::ONE),
            ..Default::default()
        }),
    }
    .id();
    commands
        .get_entity(hands_resource.left.palm)
        .unwrap()
        .push_children(&[spell_ind_id]);
}

fn despawn_spell_indicator(
    mut commands: Commands,
    indicator_query: Query<Entity, With<SpellIndicator>>,
) {
    for indicator in indicator_query.iter() {
        commands.entity(indicator).despawn_recursive();
    }
}

fn queue_new_spell(mut spell_queue: ResMut<QueuedSpell>, selected_spell: Res<SelectedSpell>) {
    spell_queue.0 = selected_spell.0;
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
        next_spell_state.set(SpellStatus::None);
    }
}

fn spawn_new_spell_entities(
    inputs: Res<PlayerInputs<WizGgrsConfig>>,
    mut commands: Commands,
    player_objs: Query<&PlayerID, With<PlayerHead>>,
    asset_handles: Res<AssetHandles>,
) {
    for p in player_objs.iter() {
        let input = inputs[p.handle].0;
        if input.spell != 0 {
            let palm_transform = Transform {
                translation: input.right_hand_pos,
                rotation: input.right_hand_rot,
                ..default()
            };
            let spell_transform = Transform {
                translation: palm_transform.translation
                    - 0.1 * Vec3::from(palm_transform.local_y()),
                rotation: input.right_hand_rot, // TODO test if this is the right direction
                ..default()
            };
            spawn_spell_projectile(
                &mut commands,
                &input.spell.try_into().unwrap(),
                spell_transform,
                &asset_handles,
            );
        }
    }
}
