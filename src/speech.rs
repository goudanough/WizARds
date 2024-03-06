use std::sync::Arc;
#[cfg(target_os = "android")]
use std::{ffi::CString, path::Path};

use bevy::prelude::*;
use bevy_oxr::xr_input::{
    hands::{common::HandsResource, HandBone},
    trackers::OpenXRTracker,
};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat,
};
use crossbeam::queue::ArrayQueue;
use vosk::*;

const BUFFER_SIZE: usize = 10000;
pub struct SpeechPlugin;

#[derive(Resource)]
struct VoiceBuffer {
    queue: Arc<ArrayQueue<i16>>,
}

#[derive(Resource)]
pub struct VoiceClip {
    data: Vec<i16>,
}

#[derive(States, Debug, Hash, Eq, PartialEq, Clone, Default)]
pub enum RecordingStatus {
    #[default]
    Awaiting,
    Recording,
}

impl Plugin for SpeechPlugin {
    fn build(&self, app: &mut App) {
        let (voice, in_stream) = setup_voice();
        app.init_state::<RecordingStatus>()
            .insert_resource(voice)
            .insert_non_send_resource(in_stream)
            .insert_resource(VoiceClip { data: Vec::new() })
            .add_systems(OnEnter(RecordingStatus::Recording), start_voice)
            .add_systems(
                Update,
                (collect_voice, check_stop_recording).run_if(in_state(RecordingStatus::Recording)),
            )
            .add_systems(
                Update,
                (check_start_recording).run_if(in_state(RecordingStatus::Awaiting)),
            );
    }
}

fn setup_voice() -> (VoiceBuffer, cpal::Stream) {
    let queue: ArrayQueue<i16> = ArrayQueue::new(BUFFER_SIZE);
    let queue = Arc::new(queue);
    let queue2 = queue.clone();

    let callback = move |data: &[i16], _: &cpal::InputCallbackInfo| queue_input_data(data, &queue2);
    let err = move |err: cpal::StreamError| eprintln!("An error occurred on stream: {}", err);

    let host = cpal::default_host();
    let input_device = host
        .default_input_device()
        .expect("failed to find input device");
    let mut configs = input_device
        .supported_input_configs()
        .expect("error querying configs");

    let config = configs
        .find(|c| {
            c.sample_format() == SampleFormat::I16
                && c.channels() == 2
                && c.max_sample_rate() == cpal::SampleRate(44100)
        })
        .expect("no supported config.")
        .with_sample_rate(cpal::SampleRate(44100))
        .config();

    let in_stream = input_device
        .build_input_stream(&config, callback, err, None)
        .unwrap();
    in_stream.play().unwrap();

    (VoiceBuffer { queue }, in_stream)
}

fn queue_input_data(data: &[i16], queue: &Arc<ArrayQueue<i16>>) {
    for &sample in data.iter() {
        queue.force_push(sample);
    }
}

pub(crate) fn fetch_recogniser(grammar: &[impl AsRef<str>]) -> Recognizer {
    #[cfg(target_os = "android")]
    {
        if !(Path::new(
            "/storage/emulated/0/Android/data/com.github.goudanough.wizards/files/vosk-model",
        )
        .exists())
        {
            let activity = bevy::winit::ANDROID_APP.get().unwrap();
            let model_zip = activity
                .asset_manager()
                .open(&CString::new("vosk-model.zip").unwrap())
                .unwrap();
            zip::ZipArchive::new(model_zip)
                .unwrap()
                .extract("/storage/emulated/0/Android/data/com.github.goudanough.wizards/files")
                .unwrap();
        }
    }

    // Attempt to fetch model, repeat until successful.
    let model: Model = loop {
        match Model::new(
            "/storage/emulated/0/Android/data/com.github.goudanough.wizards/files/vosk-model",
        ) {
            Some(model) => break model,
            None => println!("Failed to fetch vosk model, trying again."),
        }
    };
    // Attempt to create recogniser, repeat until successful, and return.
    loop {
        if let Some(mut r) = Recognizer::new_with_grammar(&model, 44100., &grammar) {
            r.set_words(true);
            return r;
        } else {
            println!("Failed to create recogniser, trying again.")
        }
    }
}

fn start_voice(voice: Res<VoiceBuffer>) {
    let n_samples = voice.queue.len();

    for _ in 0..n_samples {
        voice.queue.pop();
    }
}

fn collect_voice(voice: Res<VoiceBuffer>, mut clip: ResMut<VoiceClip>) {
    let n_samples = voice.queue.len();

    for _ in 0..n_samples {
        clip.data.push(voice.queue.pop().unwrap());
    }
}

fn check_start_recording(
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: Res<HandsResource>,
    mut clip: ResMut<VoiceClip>,
    mut next_spell_state: ResMut<NextState<RecordingStatus>>,
) {
    let thumb_tip = hand_bones.get(hands_resource.left.thumb.tip).unwrap();
    let index_tip = hand_bones.get(hands_resource.left.index.tip).unwrap();
    let thumb_index_dist = (thumb_tip.translation - index_tip.translation).length();

    if thumb_index_dist < 0.02 {
        clip.data.clear();
        next_spell_state.set(RecordingStatus::Recording);
    }
}

fn check_stop_recording(
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: Res<HandsResource>,
    mut next_spell_state: ResMut<NextState<RecordingStatus>>,
) {
    let thumb_tip = hand_bones.get(hands_resource.left.thumb.tip).unwrap();
    let index_tip = hand_bones.get(hands_resource.left.index.tip).unwrap();
    let thumb_index_dist = (thumb_tip.translation - index_tip.translation).length();

    if thumb_index_dist > 0.02 {
        next_spell_state.set(RecordingStatus::Awaiting);
    }
}

pub fn get_recognized_words<'a>(
    clip: &VoiceClip,
    recogniser: &'a mut Recognizer,
) -> std::str::SplitWhitespace<'a> {
    let mut averaged_channel_data: Vec<i16> = Vec::with_capacity(clip.data.len() / 2);
    for index in (1..clip.data.len()).step_by(2) {
        averaged_channel_data.push((clip.data[index - 1] + clip.data[index]) / 2);
    }

    recogniser.accept_waveform(&averaged_channel_data);
    let result = recogniser
        .final_result()
        .single()
        .expect("Expect a single result, got one with alternatives");

    result.text.split_whitespace()
}
