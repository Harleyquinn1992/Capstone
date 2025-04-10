use iced::{alignment, Application, Command, Element, Length, Settings, Theme, Size, mouse, event, Event};
use iced::widget::{container, text, button, row, column};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::thread;
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};
use iced::futures::stream::StreamExt;  // Required for rx.next()
use iced::window::Level;
use iced::window;

// App state
struct SubWave {
    is_capturing: bool,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    latest_transcription: String,
    drag_origin: Option<(f64, f64)>,
}

#[derive(Debug, Clone)]
enum Message {
    StartCapture,
    StopCapture,
    TranscriptionUpdate(String),
    StartWindowDrag(f64,f64),
    DragWindow(f64, f64),
    EndWindowDrag,
    None,
}

impl Default for SubWave {
    fn default() -> Self {
        Self {
            is_capturing: false,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            latest_transcription: String::new(),
            drag_origin: None,
        }
    }
}

fn translucent_container_theme(_theme: &iced::Theme) -> iced::widget::container::Appearance {
    iced::widget::container::Appearance {
        background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.7))),
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

fn container_theme(_: &iced::Theme) -> iced::widget::container::Appearance {
    iced::widget::container::Appearance {
        background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.0))), //transparent
        border: iced::Border::default(),
        ..Default::default()
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
                    
                    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
                    
                    // Spawn audio capture thread
                    thread::spawn(move || {
                        capture_audio(audio_buffer).expect("Failed to capture audio");
                    });
    
                    // Spawn transcription thread
                    let audio_buffer_clone = self.audio_buffer.clone();
                    thread::spawn(move || {
                        transcribe_audio(audio_buffer_clone, tx);
                    });
    
                    return Command::perform(async move { rx.next().await }, |msg| msg.unwrap_or(Message::StopCapture));
                }
            }
            Message::StopCapture => {
                self.is_capturing = false;
            }
            Message::TranscriptionUpdate(text) => {
                self.latest_transcription = text;
            }
            Message::StartWindowDrag(_,_) => {
                self.drag_origin = Some((0.0,0.0)); // Reset start point
            }
            Message::DragWindow(x, y) => {
                if let Some((origin_x, origin_y)) = self.drag_origin {
                    let delta_x = x - origin_x; // Calculate the change in X
                    let delta_y = y - origin_y; // Calculate the change in Y
    
                    // Move the window to the new position
                    return iced::window::move_to(
                        iced::window::Id::MAIN,
                        iced::Point::new(delta_x as f32, delta_y as f32),
                    );                    
                }
            }
            Message::EndWindowDrag => {
                self.drag_origin = None;
            }
            Message::None => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        let subtitle_box = container(
            text(&self.latest_transcription)
                .size(28)
                .style(iced::theme::Text::Color(iced::Color::WHITE))
                .horizontal_alignment(alignment::Horizontal::Center),
        )
        .padding(15)
        .style(iced::theme::Container::Custom(Box::new(translucent_container_theme)))
        .center_x();
    
        // Toggle Button
        let toggle_button = button(
            text(if self.is_capturing { "Stop" } else { "Start" })
                .size(18)
                .horizontal_alignment(alignment::Horizontal::Center),
        )
        .on_press(if self.is_capturing {
            Message::StopCapture
        } else {
            Message::StartCapture
        });
    
        // Clear subtitles button
        let clear_button = button(text("Clear").size(18))
            .on_press(Message::TranscriptionUpdate(String::new()));
    
        // Placeholder for a settings button
        let settings_button = button(text("Settings").size(18));
    
        // Button row
        let button_row = row![
            toggle_button,
            clear_button,
            settings_button
        ]
        .spacing(15)
        .align_items(iced::Alignment::Center);
    
        // Layout
        let layout = column![
            button_row,
            subtitle_box
        ]
        .spacing(20)
        .align_items(iced::Alignment::Center)
        .padding(20);
    
        // Outer container with dark blue background
        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(iced::theme::Container::Custom(Box::new(container_theme)))
            .center_x()
            .align_y(alignment::Vertical::Bottom)
            .into()
    }           

    // fn theme(&self) -> iced::Theme {
    //     iced::Theme::Dracula
    // }
    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::event::listen().map(|event| match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                // If the left button is pressed, we need to get the current cursor position
                Message::StartWindowDrag(0.0, 0.0) // Placeholder to set on button press
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                // Update the window position while dragging
                Message::DragWindow(position.x.into(), position.y.into())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                // End dragging when the mouse button is released
                Message::EndWindowDrag
            }
            _ => Message::None,
        })
    }
}

// Audio Capture Function
fn capture_audio(audio_buffer: Arc<Mutex<Vec<f32>>>) -> Result<(), Box<dyn std::error::Error>> {
    let audio_buffer_clone = Arc::clone(&audio_buffer); // Clone it here

    let host = cpal::default_host();
    let device = host.default_input_device().expect("Failed to find input device");
    let config = device.default_input_config()?.config();

    let noise_threshold = 0.0001; // Define threshold for noise filtering

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
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(())
}

// Audio Transcription Function
fn transcribe_audio(
    audio_buffer: Arc<Mutex<Vec<f32>>>, 
    tx: iced::futures::channel::mpsc::UnboundedSender<Message>
) {
    let model_path = "models/ggml-base.en-q8_0.bin";
    let whisper_params = WhisperContextParameters::default();
    let whisper_ctx = WhisperContext::new_with_params(model_path, whisper_params)
        .expect("Failed to load Whisper model");

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_realtime(false);
    params.set_print_progress(false);
    params.set_print_timestamps(false);
    params.set_print_special(false);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(1000));

        let audio_data = {
            let mut buffer = audio_buffer.lock().unwrap();
            if buffer.is_empty() {
                continue;
            }
            std::mem::take(&mut *buffer)
        };

        let mut whisper_state = whisper_ctx.create_state().expect("Failed to create Whisper state");

        if let Err(_) = whisper_state.full(params.clone(), &audio_data) {
            continue;
        }

        let num_segments = whisper_state.full_n_segments().unwrap_or(0);
        let mut transcription = String::new();

        for i in 0..num_segments {
            if let Ok(text) = whisper_state.full_get_segment_text(i) {
                transcription.push_str(&text);
                transcription.push(' ');
            }
        }

        if !transcription.is_empty() {
            let _ = tx.unbounded_send(Message::TranscriptionUpdate(transcription.clone()));
        }
    }
}


pub fn main() -> iced::Result {
    SubWave::run(Settings {
        window: iced::window::Settings {
            size: Size::new(800.0,170.0),
            decorations: false,    // Remove window frame
            transparent: true,     // Make window background transparent
            level: Level::AlwaysOnTop,
            ..Default::default()
        },
        ..Default::default()
    })
}
