use bevy::prelude::*;
use vosk::*;
use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, SampleFormat, StreamConfig};
use std::sync::Arc;
use crossbeam::queue::ArrayQueue;

const BUFFER_SIZE: usize = 10000;

pub struct SpeechPlugin;

#[derive(Resource)]
struct SpeechRecogniser(Recognizer);

#[derive(Resource)]
struct VoiceBuffer {
    queue: Arc<ArrayQueue<i16>>,
}
#[derive(Resource)]
struct VoiceClip{
    data: Vec<i16>
}

impl Plugin for SpeechPlugin {
    fn build(&self, app: &mut App) {
        let (voice, in_stream) = setup_voice();
        app
        .insert_resource(SpeechRecogniser(fetch_recogniser()))
        .insert_resource(voice)
        .insert_non_send_resource(in_stream)
        .insert_resource(VoiceClip{data:Vec::new()})
        .add_systems(Update, handle_voice);
    }
}

fn setup_voice() -> (VoiceBuffer, cpal::Stream) {
    let queue: ArrayQueue<i16> = ArrayQueue::new(BUFFER_SIZE);
    let queue = Arc::new(queue);
    let queue2 =  queue.clone();

    let callback = move |data: &[i16], _: &cpal::InputCallbackInfo| {
        queue_input_data(data, &queue2)
    };
    let err = move |err: cpal::StreamError| {eprintln!("An error occurred on stream: {}",err)};

    let host = cpal::default_host();
    let input_device = host.default_input_device().expect("failed to find input device");
    let mut configs = input_device.supported_input_configs()
    .expect("error querying configs");
    let config = configs.find(|c| c.sample_format() == SampleFormat::I16 && c.channels() == 1)
    .expect("no supported config.")
    .with_sample_rate(cpal::SampleRate(44100))
    .config();

    let in_stream = input_device.build_input_stream(&config, callback, err, None).unwrap();
    in_stream.play().unwrap();

    (VoiceBuffer {queue: queue}, in_stream)
}

fn queue_input_data(data: &[i16], queue: &Arc<ArrayQueue<i16>>) {
    for &sample in data.iter() {
        queue.force_push(sample);
    }
}

fn fetch_recogniser() -> Recognizer {
    let grammar = ["red", "green", "blue"];
    // Attempt to fetch model, repeat until successful.
    let model: Model = loop {
        match Model::new("src/vosk-model") {
            Some(model) => break model,
            None => println!("Failed to fetch vosk model, trying again."),
        }
    };
    // Attempt to create recogniser, repeat until successful, and return.
    loop {
        if let Some(mut r) = Recognizer::new_with_grammar(&model, 44100., &grammar) {
            r.set_words(true);
            return r;
        } else { println!("Failed to create recogniser, trying again.")}
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
            println!("Collected {} samples!", clip.data.len());
            recogniser.0.accept_waveform(&clip.data);
            clip.data.clear();
            let result: CompleteResultSingle = recogniser.0.final_result().single().expect("Expect a single result, got one with alternatives");
            if result.text != "" {
                println!("Heard {}", result.text);
            } else {
                println!("Failed to recognise word, or word is not in grammar");
            }
        }
}