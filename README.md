# CSCI 490 Capstone CSUCHICO 2025

# SubWave - Real-time Audio Transcription

### Hi
My name is Dinh Thien Tu Tran and this is my capstone project.

## Introduction

SubWave is an application designed to help individuals with hearing impairments by providing real-time subtitles for any audio played on their system. By capturing the system's audio output and using Whisper's transcription capabilities, SubWave generates live subtitles that appear on your screen as you listen to music, videos, podcasts, or any other audio.

## Features
- Real-time audio capture: SubWave listens to your system's audio and transcribes it into subtitles as you play any audio.

- Real-time transcription: As audio is captured, SubWave processes the audio using Whisper's ASR (Automatic Speech Recognition) engine and shows the transcribed text on your screen.

- Subtitles display: Subtitles are displayed in a floating window that you can drag around the screen.

- Toggle transcription: You can start or stop the transcription by clicking a toggle button.

- Clear subtitles: You can clear the subtitles at any time using a clear button.


## How SubWave Selects Audio Input

SubWave uses the WASAPI loopback method (Windows Audio Session API) provided by the cpal crate to directly capture audio from your system's output devices (such as HDMI, external speakers, or built-in speakers).

Specifically, it automatically scans available output devices and prioritizes names containing keywords:

- '"hdmi"'

- '"digital"'

- '"display"'

If no suitable HDMI or external output device is detected, SubWave gracefully defaults to your system’s primary audio output device (e.g., laptop speakers or built-in audio devices).

Example console output when running SubWave:

### Troubleshooting
If you don't see subtitles:
- Go to your system sound settings
- Enable a device like “Stereo Mix” or install a virtual audio cable

## “Why am I using device name checking?”
Currently, there’s no universal, cross-platform way to reliably detect the exact audio output device playing your system’s audio programmatically.

By explicitly checking for common keywords such as HDMI, Digital, or Display in audio device names, SubWave intelligently selects the best available output device likely capturing the desired audio stream.

This heuristic approach is lightweight, efficient, and has proven effective across most practical scenarios. Additionally, SubWave always provides a fallback to the default audio output device, ensuring robust behavior.

## Technical Details
* Framework: Rust with Iced for UI

* Audio Capture: cpal crate using WASAPI loopback

* Transcription Engine: Whisper (using whisper-rs)

* UI Design: Dracula Theme, draggable floating window for subtitles.

![Capstone Poster](<Capstone2.png>)

## How to run
I am currently using Windows 11, so the way I run the program is go to System->Sound->More Sound Settings->Recording and pick Stereo Mix as default audio device.
This ensure that SubWave will take audio data from the built-in stereo, not the built-in microphone.

![Set Default Audio](<SetAudio.png>)

Clone the repository and type this to command line:

cargo run