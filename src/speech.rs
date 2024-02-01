use bevy::prelude::*;
use vosk::*;
use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, SampleFormat, StreamConfig, SupportedInputConfigs, SupportedStreamConfig};
use std::{env, sync::Arc};
use crossbeam::queue::ArrayQueue;

use crate::{RecordingStatus};


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
    data: Vec<i16>,
    sample_bool: bool
}

impl Plugin for SpeechPlugin {
    fn build(&self, app: &mut App) {
        let (voice, in_stream) = setup_voice();
        app
        .insert_resource(SpeechRecogniser(fetch_recogniser()))
        .insert_resource(voice)
        .insert_non_send_resource(in_stream)
        .insert_resource(VoiceClip{data:Vec::new(), sample_bool:true})
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

    let config = configs.find(|c| c.sample_format() == SampleFormat::I16 && c.channels() == 2 && c.max_sample_rate() == cpal::SampleRate(44100))
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
    //let path = env::current_dir().unwrap();
    //println!("The current directory is {}", path.display());
    let grammar = ["red", "green", "blue"];
    // Attempt to fetch model, repeat until successful.
    
    let model: Model = loop {
        match Model::new("/storage/emulated/0/Android/data/org.goudanough.wizARds/files/vosk-model") {
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


fn handle_voice(
    keyboard_input: Res<Input<KeyCode>>, 
    voice: Res<VoiceBuffer>, 
    mut recogniser: ResMut<SpeechRecogniser>, 
    mut clip: ResMut<VoiceClip>,
    mut recording_status: ResMut<RecordingStatus>)
    {
        if recording_status.just_started {
            recording_status.just_started = false;
            println!("Start collecting voice.");
            //clip.sample_bool = true;
            let n_samples = voice.queue.len();
            // Flush the queue of samples taken before the voice button was pressed.
            for _ in 0..n_samples {
                voice.queue.pop();
            }
            recording_status.recording = true;

        }
        if recording_status.recording {
            // Get the number of samples in the queue, then add that many to the voice clip.
            // Don't keep taking until empty, as the queue will continue to fill up as samples are extracted.
            // TODO Above logic might be wrong / not be a problem, maybe change this - don't want to block on this.
           // println!("Currently collecting voice.");
            let n_samples = voice.queue.len();
            for _ in 0..n_samples {
                //if clip.sample_bool {
                    clip.data.push(voice.queue.pop().unwrap());
            //}
            //clip.sample_bool = !clip.sample_bool;

        }
        }
        if recording_status.recording && recording_status.just_ended {
            recording_status.just_ended = false;
            recording_status.recording = false;
            println!("Finished collecting voice.");
            // Pass data to recogniser
            println!("Collected {} samples!", clip.data.len());
            let mut single_channel_data:  Vec<i16> = Vec::new();
            for (index, element) in clip.data.iter().enumerate(){
                if index % 2 == 0{
                  single_channel_data.push(*element);
                }
            }

            recogniser.0.accept_waveform(&single_channel_data);
            clip.data.clear();
            let result: CompleteResultSingle = recogniser.0.final_result().single().expect("Expect a single result, got one with alternatives");
            if result.text != "" {
                println!("Heard {}", result.text);
               } else {
                println!("Failed to recognise word, or word is not in grammar");
            }
        }
}