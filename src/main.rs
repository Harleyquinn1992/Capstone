use iced::{alignment, Application, Command, Element, Length, Settings, Theme, Padding};
use iced::widget::{container, text, button, row, column};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::thread;

// App state
struct SubWave {
    is_capturing: bool,          // Tracks if audio capture is on/off
    audio_buffer: Arc<Mutex<Vec<f32>>>, // Stores captured audio data
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
                    let audio_buffer = self.audio_buffer.clone();
                    thread::spawn(move || {
                        capture_audio(audio_buffer).expect("Failed to capture audio");
                    });
                }
            }
            Message::StopCapture => {
                self.is_capturing = false;
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        container(
            column![
                text("SubWave"),
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
    let host = cpal::default_host();
    let device = host.default_input_device().expect("Failed to find input device");
    let config = device.default_input_config()?.config();

    let noise_threshold = 0.0001; // Define threshold for noise filtering

    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buffer = audio_buffer.lock().unwrap();
            
            // Filter out values below threshold
            let filtered_samples: Vec<f32> = data
                .iter()
                .cloned()
                .filter(|&sample| sample.abs() > noise_threshold) // Remove near-zero noise
                .collect();

            if !filtered_samples.is_empty() {
                buffer.extend(filtered_samples.iter());
                println!("Captured {} meaningful samples: {:?}", filtered_samples.len(), &filtered_samples[..filtered_samples.len().min(10)]);
            }
        },
        |err| eprintln!("Stream error: {}", err),
        None, // Latency is set to default
    )?;

    stream.play()?;
    
    // Keep the stream alive
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    // Now `audio_buffer` is still valid here
    loop {
        let buffer = audio_buffer.lock().unwrap();
        if buffer.is_empty() {
            break;
        }
    }

    Ok(())
}

pub fn main() -> iced::Result {
    SubWave::run(Settings::default())
}