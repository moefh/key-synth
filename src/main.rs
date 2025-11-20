mod midi_message;
mod midi_reader;
mod midi_ports;
mod audio_writer;
mod synth;
mod synth_voice;
mod keyboard;
mod app;
mod show_error;

use std::sync::mpsc;

use midi_message::MidiMessage;
use midi_reader::MidiReaderCommand;
use audio_writer::{AudioWriter, RequestedConfig};

const DEFAULT_SLEEP_TIME: u64 = 5000;
const DEFAULT_MIDI_PORTS: &[&str] = &[
    // MIDI inputs we'll try to connect to at startup (the actual port
    // name just has to contain the text here to be considered):
    "SMK25",
];
const PREF_SOUND_CONFIG: RequestedConfig = RequestedConfig {
    min_sample_rate: 44100,
    max_sample_rate: 48000,
    pref_sample_rate: 48000,
    buffer_size: 1024,
    num_channels: 2,
};

fn start_app(audio_writer: AudioWriter, midi_write: mpsc::Sender<MidiMessage>, midi_read: mpsc::Receiver<MidiMessage>,
             reader_command: Option<mpsc::Sender<MidiReaderCommand>>) -> eframe::Result {
    let viewport = egui::ViewportBuilder::default().with_inner_size([1800.0, 350.0]).with_min_inner_size([640.0, 236.0]);
    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "Key Synth",
        options,
        Box::new(|cc| {
            Ok(Box::new(app::KeySynthApp::new(cc, audio_writer, midi_read, midi_write, reader_command)))
        })
    )
}

fn show_error(message: String) -> eframe::Result {
    let viewport = egui::ViewportBuilder::default().with_inner_size([640.0, 480.0]);
    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "Key Synth - Error",
        options,
        Box::new(|cc| {
            Ok(Box::new(show_error::ShowErrorApp::new(cc, message)))
        })
    )
}

fn main() -> eframe::Result {
    // MIDI messages are written to `midi_write` by the UI and the
    // midi reader, and read from `midi_read` by the synth.
    let (midi_write, midi_read) = mpsc::channel::<MidiMessage>();

    // The midi reader receives events from the selected MIDI IN
    // port and writes midi messages to `midi_write`.  We control
    // it (configure/stop) by writing comands to `reader_command`.
    let reader_command = midi_reader::start(DEFAULT_SLEEP_TIME, DEFAULT_MIDI_PORTS, midi_write.clone()).ok();

    // The audio writer requests samples from the synth and
    // sends audio to the output device. It will be started by the App.
    let audio_writer = AudioWriter::init(PREF_SOUND_CONFIG);

    match audio_writer {
        Ok(audio_writer) => { start_app(audio_writer, midi_write, midi_read, reader_command) }
        Err(e) => { show_error(format!("Error initializing sound: {}", e)) }
    }
}
