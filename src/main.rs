use iced::{alignment, Application, Command, Element, Length, Settings, Theme, Padding};
use iced::widget::{container, text, button, row, column};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::thread;
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};
use vosk::{Model, Recognizer};
use serde_json;

// App state
struct SubWave {
    is_capturing: bool,          // Tracks if audio capture is on/off
    audio_buffer: Arc<Mutex<Vec<f32>>>, // Stores captured audio data
    transcribed_text: Arc<Mutex<String>>, // Stores real-time transcription
}

#[derive(Debug, Clone)]
enum Message {
    StartCapture,
    StopCapture,
}

impl Default for SubWave {
    fn default() -> Self {
        Self {
            is_capturing: false,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            transcribed_text: Arc::new(Mutex::new(String::new())),
        }
    }
}

impl Application for SubWave {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (Self::default(), Command::none())
    }

    fn title(&self) -> String {
        String::from("SubWave")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::StartCapture => {
                if !self.is_capturing {
                    self.is_capturing = true;
    
                    let audio_buffer_clone1 = Arc::clone(&self.audio_buffer);
                    let audio_buffer_clone2 = Arc::clone(&self.audio_buffer);
                    let transcribed_text_clone = Arc::clone(&self.transcribed_text);
    
                    thread::spawn(move || {
                        capture_audio(audio_buffer_clone1).expect("Failed to capture audio");
                    });
    
                    thread::spawn(move || {
                        transcribe_audio_vosk(audio_buffer_clone2, transcribed_text_clone);
                    });
                }
            }
            Message::StopCapture => {
                self.is_capturing = false; // Stop capturing audio
            }
        }
        Command::none()
    }    

    fn view(&self) -> Element<Self::Message> {
        let transcribed_text = self.transcribed_text.lock().unwrap().clone();
        
        container(
            column![
                text("SubWave").size(30),
                text(transcribed_text).size(20), // Display transcribed text
                row![
                    button("On").on_press(Message::StartCapture),
                    button("Off").on_press(Message::StopCapture),
                ]
                .spacing(30)
                .padding(Padding::from(30))
            ]
            .align_items(iced::Alignment::Center),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
    }    

    fn theme(&self) -> iced::Theme {
        iced::Theme::Dracula
    }
}

// Audio Capture Function
fn capture_audio(audio_buffer: Arc<Mutex<Vec<f32>>>) -> Result<(), Box<dyn std::error::Error>> {
    let audio_buffer_clone = Arc::clone(&audio_buffer); // Clone it here

    let host = cpal::default_host();
    let device = host.default_input_device().expect("Failed to find input device");
    let config = device.default_input_config()?.config();

    let noise_threshold = 0.001; // Define threshold for noise filtering

    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buffer = audio_buffer_clone.lock().unwrap();
            
            // Filter out values below threshold and extend the buffer
            buffer.extend(data.iter().cloned().filter(|&sample| sample.abs() > noise_threshold));
        },
        |err| eprintln!("Stream error: {}", err),
        None, // Latency is set to default
    )?;

    stream.play()?;

    // Keep the stream alive while capturing audio
    std::thread::sleep(std::time::Duration::from_secs(10)); // Keep the stream alive for a while
    Ok(())
}

// Audio Transcription Function
fn transcribe_audio(audio_buffer: Arc<Mutex<Vec<f32>>>) {
    let model_path = "models/ggml-base.en.bin";

    // Create Whisper context parameters
    let whisper_params = WhisperContextParameters::default();
    let whisper_ctx = WhisperContext::new_with_params(model_path, whisper_params)
        .expect("Failed to load Whisper model");

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    // Set parameters individually (no chaining)
    params.set_print_realtime(false);
    params.set_print_progress(false);
    params.set_print_timestamps(false);
    params.set_print_special(false); // Suppress special tokens

    loop {
        std::thread::sleep(std::time::Duration::from_secs(3));

        let audio_data = {
            let mut buffer = audio_buffer.lock().unwrap();
            if buffer.is_empty() {
                continue;
            }
            std::mem::take(&mut *buffer)
        };

        let mut whisper_state = whisper_ctx.create_state().expect("Failed to create Whisper state");

        // Clone `params` before passing it to `full()`, since `full()` consumes it
        if let Err(_) = whisper_state.full(params.clone(), &audio_data) {
            continue; // Skip any transcription errors without logging
        }

        let num_segments = whisper_state.full_n_segments().unwrap_or(0);
        for i in 0..num_segments {
            if let Ok(text) = whisper_state.full_get_segment_text(i) {
                println!("o0 {} 0o", text); // Print only transcribed text
            }
        }
    }
}

fn transcribe_audio_vosk(audio_buffer: Arc<Mutex<Vec<f32>>>, transcribed_text: Arc<Mutex<String>>) {
    let model_path = "models/vosk-model-en-us-0.22"; 
    let model = Model::new(model_path).expect("Failed to load Vosk model");
    let mut recognizer = Recognizer::new(&model, 16000.0).expect("Failed to create recognizer");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(2));

        let audio_data = {
            let mut buffer = audio_buffer.lock().unwrap();
            if buffer.is_empty() {
                continue;
            }
            let data_f32 = std::mem::take(&mut *buffer);
            data_f32.iter().map(|&sample| (sample * i16::MAX as f32) as i16).collect::<Vec<i16>>() // Convert f32 to i16
        };

        // Process audio through Vosk
        if let Ok(_) = recognizer.accept_waveform(&audio_data) {
            let result_json = serde_json::to_string(&recognizer.final_result().multiple().unwrap()).unwrap();
            if let Ok(result) = serde_json::from_str::<serde_json::Value>(&result_json) {
                if let Some(text) = result["text"].as_str() {
                    let mut transcribed = transcribed_text.lock().unwrap();
                    *transcribed = text.to_string();
                }
            }
        }
    }
}

pub fn main() -> iced::Result {
    SubWave::run(Settings::default())
}