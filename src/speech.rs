#[cfg(target_os = "android")]
use bevy::winit::android_activity::AndroidApp;
#[cfg(target_os = "android")]
use std::ffi::CString;
#[cfg(target_os = "android")]
use std::path::Path;
#[cfg(target_os = "android")]
use zip::ZipArchive;

use bevy::prelude::*;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat,
};

use crossbeam::queue::ArrayQueue;
use std::sync::Arc;
use vosk::*;

use crate::spell_control::{Spell, SpellStatus, SpellType};

const BUFFER_SIZE: usize = 10000;

pub struct SpeechPlugin;

#[derive(Resource)]
struct SpeechRecogniser(Recognizer);

#[derive(Resource)]
struct VoiceBuffer {
    queue: Arc<ArrayQueue<i16>>,
}
#[derive(Resource)]
struct VoiceClip {
    data: Vec<i16>,
}

impl Plugin for SpeechPlugin {
    fn build(&self, app: &mut App) {
        let (voice, in_stream) = setup_voice();
        app.insert_resource(SpeechRecogniser(fetch_recogniser()))
            .insert_resource(voice)
            .insert_non_send_resource(in_stream)
            .insert_resource(VoiceClip { data: Vec::new() })
            .insert_resource(RecordingStatus {
                just_started: false,
                recording: false,
                just_ended: false,
            })
            .add_systems(Update, handle_voice);
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

fn fetch_recogniser() -> Recognizer {
    let grammar = ["red", "green", "blue"];

    #[cfg(target_os = "android")]
    {
        if !(Path::new("/storage/emulated/0/Android/data/org.goudanough.wizARds/files/vosk-model")
            .exists())
        {
            let activity = bevy::winit::ANDROID_APP.get().unwrap();
            let model_zip = activity
                .asset_manager()
                .open(&CString::new("vosk-model.zip").unwrap())
                .unwrap();
            zip::ZipArchive::new(model_zip)
                .unwrap()
                .extract("/storage/emulated/0/Android/data/org.goudanough.wizARds/files");
        }
    }

    // Attempt to fetch model, repeat until successful.
    let model: Model = loop {
        match Model::new("/storage/emulated/0/Android/data/org.goudanough.wizARds/files/vosk-model")
        {
            Some(model) => break model,
            None => println!("Failed to fetch vosk model, trying again."),
        }
    };
    // Attempt to create recogniser, repeat until successful, and return.
    loop {
        if let Some(mut r) = Recognizer::new_with_grammar(&model, 44100., &grammar) {
            r.set_words(true);
            return r;
        }
    }
}

#[derive(Resource)]
pub struct RecordingStatus {
    pub just_started: bool,
    pub recording: bool,
    pub just_ended: bool,
}

fn handle_voice(
    voice: Res<VoiceBuffer>,
    mut recogniser: ResMut<SpeechRecogniser>,

    mut clip: ResMut<VoiceClip>,
    mut recording_status: ResMut<RecordingStatus>,
    mut spell: ResMut<Spell>,
) {
    if recording_status.just_started {
        recording_status.just_started = false;
        spell.status = SpellStatus::None;

        //clip.sample_bool = true;
        let n_samples = voice.queue.len();
        // Flush the queue of samples taken before the voice button was pressed.
        for _ in 0..n_samples {
            voice.queue.pop();
        }
    }
    if recording_status.recording {
        // Get the number of samples in the queue, then add that many to the voice clip.
        // Don't keep taking until empty, as the queue will continue to fill up as samples are extracted.
        // TODO Above logic might be wrong / not be a problem, maybe change this - don't want to block on this.
        let n_samples = voice.queue.len();
        for _ in 0..n_samples {
            clip.data.push(voice.queue.pop().unwrap());
        }
    }
    if recording_status.just_ended {
        recording_status.just_ended = false;
        recording_status.recording = false;
        // Pass data to recogniser
        let mut averaged_channel_data: Vec<i16> = Vec::new();
        for index in (1..clip.data.len()).step_by(2) {
            averaged_channel_data.push((clip.data[index - 1] + clip.data[index]) / 2);
        }
        recogniser.0.accept_waveform(&averaged_channel_data);
        clip.data.clear();
        let result: CompleteResultSingle = recogniser
            .0
            .final_result()
            .single()
            .expect("Expect a single result, got one with alternatives");
        process_text(result.text, spell);
    }
}

fn process_text(text: &str, mut spell: ResMut<Spell>) {
    let last_recognised_word = text.split_whitespace().last().unwrap_or("");

    match last_recognised_word {
        "red" => {
            spell.spell_type = SpellType::Red;
            spell.status = SpellStatus::Prepare;
        }
        "blue" => {
            spell.spell_type = SpellType::Blue;
            spell.status = SpellStatus::Prepare;
        }
        "green" => {
            spell.spell_type = SpellType::Green;
            spell.status = SpellStatus::Prepare;
        }
        _ => spell.status = SpellStatus::None,
    }
}
