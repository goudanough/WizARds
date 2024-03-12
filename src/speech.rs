#[cfg(target_os = "android")]
use std::{ffi::CString, path::Path};
use std::{sync::Arc, time::Duration};

use bevy::prelude::*;
use bevy_oxr::xr_input::{
    hands::{common::HandsResource, HandBone},
    trackers::OpenXRTracker,
};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    InputCallbackInfo, SampleFormat,
};
use crossbeam::queue::ArrayQueue;
use vosk::*;

const BUFFER_SIZE: usize = 10000;
pub struct SpeechPlugin;

#[derive(Resource)]
pub struct SpeechRecognizer(pub Recognizer);

#[derive(Resource, Clone)]
struct VoiceBuffer(Arc<ArrayQueue<i16>>);

impl Default for VoiceBuffer {
    fn default() -> Self {
        Self(Arc::new(ArrayQueue::new(BUFFER_SIZE)))
    }
}

#[derive(Resource)]
pub struct RecognizedWord(pub String);

#[derive(States, Debug, Hash, Eq, PartialEq, Clone, Default)]
pub enum RecordingStatus {
    #[default]
    Awaiting,
    Recording,
    Success,
}

#[derive(Resource)]
pub struct RecognitionTimer {
    timer: Timer,
}

impl Plugin for SpeechPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<RecordingStatus>()
            .insert_resource(RecognizedWord("".to_owned()))
            .add_systems(Startup, setup_audio)
            .add_systems(OnEnter(RecordingStatus::Recording), clear_audio_buf)
            .add_systems(OnExit(RecordingStatus::Recording), reset_recognizer)
            .add_systems(
                OnEnter(RecordingStatus::Success),
                |mut state: ResMut<NextState<RecordingStatus>>| {
                    state.set(RecordingStatus::Awaiting)
                },
            )
            .add_systems(
                Update,
                (submit_recorded_buf, check_stop_recording, check_word_found)
                    .run_if(in_state(RecordingStatus::Recording)),
            )
            .add_systems(
                Update,
                (check_start_recording).run_if(in_state(RecordingStatus::Awaiting)),
            );
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
            None => eprintln!("Failed to fetch vosk model, trying again."),
        }
    };
    // Attempt to create recogniser, repeat until successful, and return.
    loop {
        if let Some(mut r) = Recognizer::new_with_grammar(&model, 44100., grammar) {
            r.set_words(true);
            return r;
        } else {
            eprintln!("Failed to create recogniser, trying again.")
        }
    }
}

fn reset_recognizer(recognizer: Option<ResMut<SpeechRecognizer>>) {
    if let Some(mut r) = recognizer {
        r.0.reset()
    }
}

fn setup_audio(world: &mut World) {
    let voice_buf = VoiceBuffer::default();
    world.insert_resource(voice_buf.clone());

    let on_input = move |data: &[i16], _: &InputCallbackInfo| queue_input_data(data, &voice_buf.0);
    let on_err = move |err| eprintln!("An error occurred on stream: {err}");

    let input_device = cpal::default_host()
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

    let audio_stream = input_device
        .build_input_stream(&config, on_input, on_err, None)
        .unwrap();
    audio_stream.play().unwrap();
    world.insert_non_send_resource(audio_stream);
}

fn queue_input_data(data: &[i16], queue: &Arc<ArrayQueue<i16>>) {
    for &sample in data.iter() {
        queue.force_push(sample);
    }
}

fn clear_audio_buf(voice_buffer: ResMut<VoiceBuffer>) {
    while voice_buffer.0.pop().is_some() {}
}

fn submit_recorded_buf(
    voice_buffer: ResMut<VoiceBuffer>,
    recognizer: Option<ResMut<SpeechRecognizer>>,
) {
    let Some(mut recognizer) = recognizer else {
        return;
    };
    let buf = &*voice_buffer.0;

    let mut averaged_channel_data: Vec<i16> = Vec::with_capacity(buf.len() / 2);
    while let (Some(l), Some(r)) = (buf.pop(), buf.pop()) {
        averaged_channel_data.push((l + r) / 2);
    }
    recognizer.0.accept_waveform(&averaged_channel_data);
}

fn check_start_recording(
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: Res<HandsResource>,
    mut recording_state: ResMut<NextState<RecordingStatus>>,
) {
    if check_fingers_close(hand_bones, &hands_resource) {
        recording_state.set(RecordingStatus::Recording);
    }
}

fn check_stop_recording(
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: Res<HandsResource>,
    mut recording_state: ResMut<NextState<RecordingStatus>>,
) {
    if !check_fingers_close(hand_bones, &hands_resource) {
        recording_state.set(RecordingStatus::Awaiting);
    }
}

fn check_word_found(
    recognizer: Option<ResMut<SpeechRecognizer>>,
    mut recording_state: ResMut<NextState<RecordingStatus>>,
    mut word: ResMut<RecognizedWord>,
) {
    let Some(mut recognizer) = recognizer else {
        return;
    };
    let partial = recognizer.0.partial_result().partial;
    let last_word = partial.split_whitespace().last();
    let Some(last_word) = last_word else { return };
    *word = RecognizedWord(last_word.to_string());
    recognizer.0.reset();
    recording_state.set(RecordingStatus::Success);
}

fn check_fingers_close(
    hand_bones: Query<&Transform, (With<OpenXRTracker>, With<HandBone>)>,
    hands_resource: &HandsResource,
) -> bool {
    let thumb_dists = (hand_bones
        .get(hands_resource.left.thumb.tip)
        .unwrap()
        .translation
        - hand_bones
            .get(hands_resource.right.thumb.tip)
            .unwrap()
            .translation)
        .length();

    let index_dists = (hand_bones
        .get(hands_resource.left.index.tip)
        .unwrap()
        .translation
        - hand_bones
            .get(hands_resource.right.index.tip)
            .unwrap()
            .translation)
        .length();

    let mut spell_check_close = true;

    for dist in [thumb_dists, index_dists] {
        spell_check_close = spell_check_close && dist < 0.25;
    }

    spell_check_close
}
