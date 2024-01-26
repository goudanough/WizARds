use bevy::prelude::*;
use vosk::*;

const SAMPLE_RATE: f32 = 16000.;

pub struct SpeechPlugin;

#[derive(Resource)]
struct SpeechRecogniser(Recognizer);

#[derive(Resource)]
struct VoiceClip {
    active: bool,
}

impl Plugin for SpeechPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(SpeechRecogniser(fetch_recogniser()))
        .insert_resource(VoiceClip {active: false})
        .add_systems(Update, handle_voice);
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

fn handle_voice(keyboard_input: Res<Input<KeyCode>>, clip: Res<VoiceClip>, recogniser: Res<SpeechRecogniser>){
    if !clip.active {
        if keyboard_input.just_pressed(KeyCode::V) {
            println!("Start collecting voice.");
        }
        if keyboard_input.pressed(KeyCode::V) {
            println!("Still collecting voice.");
        }
        if keyboard_input.just_released(KeyCode::V) {
            println!("Finished collecting voice.")
        }
    }
}





