use bevy::{prelude::*, reflect::Array};
use vosk::*;
use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, Stream, StreamConfig};
use std::sync::Arc;
use crossbeam::queue::ArrayQueue;

const SAMPLE_RATE: f32 = 16000.;

pub struct SpeechPlugin;

#[derive(Resource)]
struct SpeechRecogniser(Recognizer);

#[derive(Resource)]
struct VoiceBuffer {
    queue: Arc<ArrayQueue<f32>>,
}
#[derive(Resource)]
struct VoiceClip{
    data: Vec<f32>
}

impl Plugin for SpeechPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(SpeechRecogniser(fetch_recogniser()))
        .insert_resource(setup_voice())
        .insert_resource(VoiceClip{data:Vec::new()})
        .add_systems(Update, handle_voice);
    }
}

fn setup_voice() -> VoiceBuffer {
    let queue: ArrayQueue<f32> = ArrayQueue::new(1000);
    let queue = Arc::new(queue);
    let queue2 =  queue.clone();

    let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        queue_input_data(data, &queue2)
    };
    let err = move |err: cpal::StreamError| {eprintln!("An error occurred on stream: {}",err)};

    let host = cpal::default_host();
    let input_device = host.default_input_device().expect("failed to find input device");
    let config: StreamConfig = input_device.default_input_config().unwrap().into();
    let in_stream = input_device.build_input_stream(&config, callback, err, None).unwrap();
    in_stream.play().unwrap();

    VoiceBuffer {queue: queue}
}

fn queue_input_data(data: &[f32], queue: &Arc<ArrayQueue<f32>>) {
    for &sample in data.iter() {
        queue.force_push(sample);
    }
}

fn fetch_recogniser() -> Recognizer {
    let grammar = ["red", "green", "blue", "[unk]"];
    // Attempt to fetch model, repeat until successful.
    let model: Model = loop {
        match Model::new("src/vosk-model") {
            Some(model) => break model,
            None => println!("Failed to fetch vosk model, trying again."),
        }
    };
    // Attempt to create recogniser, repeat until successful, and return.
    loop {
        match Recognizer::new_with_grammar(&model, SAMPLE_RATE, &grammar) {
            Some(recogniser) => return recogniser,
            None => println!("Failed to create recogniser, trying again."),
        }
    }
}

fn handle_voice(keyboard_input: Res<Input<KeyCode>>, voice: Res<VoiceBuffer>, mut recogniser: ResMut<SpeechRecogniser>, mut clip: ResMut<VoiceClip>){
        if keyboard_input.just_pressed(KeyCode::V) {
            println!("Start collecting voice.");
            let n_samples = voice.queue.len();
            // Flush the queue of samples taken before the voice button was pressed.
            for _ in 0..n_samples {
                voice.queue.pop();
            }

        }
        if keyboard_input.pressed(KeyCode::V) {
            println!("Still collecting voice.");

            // Get the number of samples in the queue, then add that many to the voice clip.
            // Don't keep taking until empty, as the queue will continue to fill up as samples are extracted.
            // TODO Above logic might be wrong / not be a problem, maybe change this - don't want to block on this.
            let n_samples = voice.queue.len();
            for _ in 0..n_samples {
                clip.data.push(voice.queue.pop().unwrap());
            }
        }
        if keyboard_input.just_released(KeyCode::V) {
            println!("Finished collecting voice.");
            // Pass data to recogniser
            clip.data.clear();
        }
}





