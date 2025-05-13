use iced::{alignment, Application, Command, Element, Length, Settings, Theme, Size, mouse, Event};
use iced::widget::{container, text, button, row, column};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::thread;
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};
use iced::futures::stream::StreamExt;  // Required for rx.next()
use iced::window::Level;

// App state
struct SubWave {
    is_capturing: bool,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    latest_transcription: String,
    drag_origin: Option<(f64, f64)>,
    last_cursor_position: Option<(f64, f64)>,
    window_position: Option<(f32, f32)>,
}

#[derive(Debug, Clone)]
enum Message {
    StartCapture,
    StopCapture,
    TranscriptionUpdate(String),
    UpdateCursorPosition(f64, f64),
    StartWindowDrag,
    EndWindowDrag,
    RefreshInput,
    None,
}

impl Default for SubWave {
    fn default() -> Self {
        Self {
            is_capturing: false,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            latest_transcription: String::new(),
            drag_origin: None,
            last_cursor_position: None,
            window_position: Some((0.0, 0.0)),
        }
    }
}

fn container_theme(_: &iced::Theme) -> iced::widget::container::Appearance {
    iced::widget::container::Appearance {
        background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.8))),
        border: iced::Border {
            radius: 20.0.into(),
            width: 0.0,
            color: iced::Color::TRANSPARENT,
        },
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
            Message::UpdateCursorPosition(x, y) => {
                if let Some((start_x, start_y)) = self.drag_origin {
                    let delta_x = x - start_x;
                    let delta_y = y - start_y;
            
                    // Apply delta to previous offset
                    let (ox, oy) = self.window_position.unwrap_or((0.0, 0.0));
                    let new_x = ox + delta_x as f32;
                    let new_y = oy + delta_y as f32;
            
                    return iced::window::move_to(
                        iced::window::Id::MAIN,
                        iced::Point::new(new_x, new_y),
                    );
                }
            
                self.last_cursor_position = Some((x, y));
            }
            
            Message::StartWindowDrag => {
                self.drag_origin = self.last_cursor_position;
            }
            Message::EndWindowDrag => {
                // Update offset based on final cursor position
                if let (Some((start_x, start_y)), Some((curr_x, curr_y))) = (self.drag_origin, self.last_cursor_position) {
                    if let Some((ox, oy)) = self.window_position {
                        self.window_position = Some((
                            ox + (curr_x - start_x) as f32,
                            oy + (curr_y - start_y) as f32,
                        ));
                    }                    
                }
            
                self.drag_origin = None;
            }                      
            Message::None => {}

            Message::RefreshInput => {
                if self.is_capturing {
                    self.is_capturing = false;
            
                    // Restart the capture in a fresh thread
                    self.is_capturing = true;
            
                    let audio_buffer = self.audio_buffer.clone();
                    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
            
                    // Spawn fresh audio capture
                    thread::spawn(move || {
                        capture_audio(audio_buffer).expect("Failed to capture audio");
                    });
            
                    // Spawn transcription thread again
                    let audio_buffer_clone = self.audio_buffer.clone();
                    thread::spawn(move || {
                        transcribe_audio(audio_buffer_clone, tx);
                    });
            
                    return Command::perform(async move { rx.next().await }, |msg| msg.unwrap_or(Message::StopCapture));
                }
            }
            
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

        let refresh_button = button(text("Refresh").size(18))
            .on_press(Message::RefreshInput);
    
        // Clear subtitles button
        let clear_button = button(text("Clear").size(18))
            .on_press(Message::TranscriptionUpdate(String::new()));
    
        // Button row
        let button_row = row![
            toggle_button,
            clear_button,
            refresh_button,
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

    fn theme(&self) -> iced::Theme {
        iced::Theme::Dracula
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::event::listen().map(|event| match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                Message::UpdateCursorPosition(position.x.into(), position.y.into())             
            }            
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                Message::StartWindowDrag
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                Message::EndWindowDrag
            }
            _ => Message::None,
        })
    }    
}

// Audio Capture Function
fn capture_audio(audio_buffer: Arc<Mutex<Vec<f32>>>) -> Result<(), Box<dyn std::error::Error>> {
    let audio_buffer_clone = Arc::clone(&audio_buffer);

    let device = find_best_input_device().expect("Failed to find output device for loopback capture");
    println!("Capturing audio via WASAPI loopback from device: {}", device.name().unwrap_or("Unknown Device".to_string()));

    let config = device.default_output_config()?.config();

    let noise_threshold = 0.001; 

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buffer = audio_buffer_clone.lock().unwrap();
            
            buffer.extend(data.iter().filter(|&&sample| sample.abs() > noise_threshold));
        },
        |err| eprintln!("Stream error: {}", err),
        None, 
    )?;

    stream.play()?;

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

// Audio Transcription Function
fn transcribe_audio(
    audio_buffer: Arc<Mutex<Vec<f32>>>, 
    tx: iced::futures::channel::mpsc::UnboundedSender<Message>
) {
    let model_path = "models/ggml-base.en.bin";
    let whisper_params = WhisperContextParameters::default();
    let whisper_ctx = WhisperContext::new_with_params(model_path, whisper_params)
        .expect("Failed to load Whisper model");

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_realtime(false);
    params.set_print_progress(false);
    params.set_print_timestamps(false);
    params.set_print_special(false);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(200));

        let audio_data = {
            let mut buffer = audio_buffer.lock().unwrap();
            if buffer.len() < 8000 {
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

fn find_best_input_device() -> Option<cpal::Device> {
    let host = cpal::host_from_id(cpal::HostId::Wasapi).ok()?;

    // Keywords for external outputs
    let output_keywords = ["hdmi", "digital", "display"];

    if let Ok(devices) = host.output_devices() {
        for device in devices {
            if let Ok(name) = device.name() {
                let name_lower = name.to_lowercase();
                if output_keywords.iter().any(|kw| name_lower.contains(kw)) {
                    println!("Matched audio OUTPUT device: {}", name);
                    return Some(device);
                }
            }
        }
    }

    // Fallback to default output device if no HDMI or external match
    let default = host.default_output_device();
    if let Some(ref device) = default {
        println!("Using default audio OUTPUT device: {}", device.name().unwrap_or("Unknown".into()));
    }
    default
}

pub fn main() -> iced::Result {
    SubWave::run(Settings {
        window: iced::window::Settings {
            size: Size::new(800.0,170.0),
            decorations: false,    // Remove window frame
            level: Level::AlwaysOnTop,
            ..Default::default()
        },
        ..Default::default()
    })
}
