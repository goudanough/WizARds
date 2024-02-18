#[cfg(target_os = "android")]
use std::ffi::CString;
#[cfg(target_os = "android")]
use std::path::Path;

use bevy::prelude::*;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat,
};
use crossbeam::queue::ArrayQueue;
use std::sync::Arc;
use vosk::*;

use crate::spell_control::{SelectedSpell, Spell, SpellStatus};

const BUFFER_SIZE: usize = 10000;

pub struct SpeechPlugin;

#[derive(Resource)]
pub struct SpeechRecogniser(Recognizer);

#[derive(Resource)]
pub struct VoiceBuffer {
    queue: Arc<ArrayQueue<i16>>,
}
#[derive(Resource)]
pub struct VoiceClip {
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
            });
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
    let grammar = ["fireball", "lightning"];

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
        } else {
            println!("Failed to create recogniser, trying again.")
        }
    }
}

#[derive(Resource)]
pub struct RecordingStatus {
    pub just_started: bool,
    pub recording: bool,
    pub just_ended: bool,
}

pub fn start_voice(voice: Res<VoiceBuffer>) {
    let n_samples = voice.queue.len();

    for _ in 0..n_samples {
        voice.queue.pop();
    }
}

pub fn collect_voice(voice: Res<VoiceBuffer>, mut clip: ResMut<VoiceClip>) {
    let n_samples = voice.queue.len();

    for _ in 0..n_samples {
        clip.data.push(voice.queue.pop().unwrap());
    }
}

pub fn recognise_voice(
    mut clip: ResMut<VoiceClip>,
    mut recogniser: ResMut<SpeechRecogniser>,
    mut next_spell_state: ResMut<NextState<SpellStatus>>,
    mut selected_spell: ResMut<SelectedSpell>,
) {
    let mut averaged_channel_data: Vec<i16> = Vec::new();
    for index in (1..clip.data.len()).step_by(2) {
        averaged_channel_data.push((clip.data[index - 1] + clip.data[index]) / 2);
    }
    clip.data.clear();

    recogniser.0.accept_waveform(&averaged_channel_data);
    let result: CompleteResultSingle = recogniser
        .0
        .final_result()
        .single()
        .expect("Expect a single result, got one with alternatives");
    let last_word = result.text.split_whitespace().last().unwrap_or("");

    let (next_s, s_spell) = match last_word {
        "fireball" => (SpellStatus::Armed, Some(Spell::Fireball)),
        "lightning" => (SpellStatus::Armed, Some(Spell::Lightning)),
        _ => (SpellStatus::None, None),
    };
    next_spell_state.set(next_s);
    selected_spell.0 = s_spell;
}
