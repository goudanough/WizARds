use bevy::{prelude::*, reflect::Array};
use vosk::*;
use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, Stream};
use std::sync::Arc;
use crossbeam::queue::ArrayQueue;

const SAMPLE_RATE: f32 = 16000.;

pub struct SpeechPlugin;

#[derive(Resource)]
struct SpeechRecogniser(Recognizer);

#[derive(Resource)]
struct VoiceHandle {
    buffer: Vec<i16>,
    queue: Arc<ArrayQueue<f32>>,
}

impl Plugin for SpeechPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(SpeechRecogniser(fetch_recogniser()))
        .insert_resource(setup_voice())
        .add_systems(Update, handle_voice);
    }
}

fn setup_voice() -> VoiceHandle {
    let queue: ArrayQueue<f32> = ArrayQueue::new(1000);
    let queue = Arc::new(queue);
    let queue2 =  queue.clone();

    let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        queue_input_data(data, &queue2)
    };
    let err = move |err: cpal::StreamError| {eprintln!("An error occurred on stream: {}",err)};

    

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

fn handle_voice(keyboard_input: Res<Input<KeyCode>>, clip: Res<VoiceHandle>, _recogniser: Res<SpeechRecogniser>){
        if keyboard_input.just_pressed(KeyCode::V) {
            println!("Start collecting voice.");
            let host = cpal::default_host();
            let input_device = host.default_input_device().expect("failed to find input device");
            

        }
        if keyboard_input.pressed(KeyCode::V) {
            println!("Still collecting voice.");
        }
        if keyboard_input.just_released(KeyCode::V) {
            println!("Finished collecting voice.")
        }
}





