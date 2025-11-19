use std::sync::mpsc;

use super::midi_message::MidiMessage;
use super::midi_reader::{MidiReaderCommand, MidiReaderConfigAcceptedPorts};
use super::synth::SynthKeyboard;
use super::synth_voice::{SynthInstrument, SynthVoice};
use super::audio_writer::AudioWriter;

const DEFAULT_SLEEP_TIME: u64 = 5000;
const DEFAULT_MIDI_PORTS: &[&str] = &[
    // MIDI inputs we'll try to connect to at startup (the actual port
    // name just has to contain the text here to be considered):
    "SMK25",
];

pub struct KeySynthApp {
    _audio_writer: Option<AudioWriter>, // never used, but must be kept alive
    audio_error: Option<String>,
    midi_write: mpsc::Sender<MidiMessage>,
    reader_command: Option<mpsc::Sender<MidiReaderCommand>>,
    midi_ports: Option<super::midi_ports::MidiPorts>,
    synth: SynthKeyboard,
    keyboard_state: super::keyboard::KeyboardState,
    volume: f32,
}

impl KeySynthApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        // MIDI messages are written to `midi_write` by the UI and the
        // midi reader, and read from `midi_read` by the synth.
        let (midi_write, midi_read) = mpsc::channel::<MidiMessage>();

        // The midi reader receives events from the selected MIDI IN
        // port and writes midi messages to `midi_write`.  We control
        // it (configure/stop) by writing comands to `reader_command`.
        let reader_command = super::midi_reader::start(DEFAULT_SLEEP_TIME, DEFAULT_MIDI_PORTS, midi_write.clone()).ok();

        // The synth reads midi messages from `midi_read` and
        // generates sound as appropriate.
        let synth = SynthKeyboard::start(midi_read, cc.egui_ctx.clone());

        // The audio writer requests samples from the synth and
        // sends audio to the output device.
        let (_audio_writer, audio_error) = match AudioWriter::start(SynthVoice::SAMPLE_RATE, SynthVoice::BUFFER_SIZE, synth.get_player()) {
            Ok(audio_writer) => (Some(audio_writer), None),
            Err(e) => (None, Some(format!("Error setting up audio output: {}", e))),
        };

        let volume = synth.get_volume();

        egui_extras::install_image_loaders(&cc.egui_ctx);
        //cc.egui_ctx.set_theme(egui::ThemePreference::Light);
        cc.egui_ctx.set_zoom_factor(1.5);
        KeySynthApp {
            _audio_writer,
            audio_error,
            synth,
            midi_write,
            reader_command,
            midi_ports: super::midi_ports::MidiPorts::open(),
            keyboard_state: super::keyboard::KeyboardState::new(),
            volume,
        }
    }

    pub fn close_midi_reader(&self) {
        if let Some(command) = &self.reader_command {
            command.send(MidiReaderCommand::Close).unwrap_or(());
        }
    }

    pub fn select_midi_in_port(&self, port: String) {
        if let Some(command) = &self.reader_command {
            let cfg = MidiReaderConfigAcceptedPorts { accepted_midi_ports: vec![port] };
            command.send(MidiReaderCommand::ConfigAcceptedPorts(cfg)).unwrap_or(());
        }
    }

    fn update_menu(&mut self, ctx: &egui::Context) {
        let mut select_midi_in_port = None;
        egui::TopBottomPanel::top("main_menu").show(ctx, |ui| {
            let quit_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Q);
            if ui.input_mut(|i| i.consume_shortcut(&quit_shortcut)) {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Synth", |ui| {
                    if ui.button("Piano").clicked() {
                        self.synth.set_instrument(SynthInstrument::PIANO);
                    }
                    if ui.button("Vibraphone").clicked() {
                        self.synth.set_instrument(SynthInstrument::VIBRAPHONE);
                    }
                    if ui.button("Bell").clicked() {
                        self.synth.set_instrument(SynthInstrument::BELL);
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        self.close_midi_reader();
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                if self.reader_command.is_some() && let Some(midi_ports) = &mut self.midi_ports {
                    ui.menu_button("Midi In", |ui| {
                        for port in midi_ports.read_port_names() {
                            if ui.button(port).clicked() {
                                select_midi_in_port = Some(port.to_owned());
                            }
                        }
                    });
                }
            });
        });
        if let Some(port) = select_midi_in_port {
            self.select_midi_in_port(port);
        }
    }

    fn update_footer(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.add_space(2.0);
            if self.synth.is_midi_connected() {
                ui.label("MIDI input connected");
            } else {
                ui.label("MIDI input not connected");
            }
        });
    }

    fn update_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().slider_width = ui.available_height();
                let mut volume = self.volume;
                ui.add(egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false).vertical());
                if self.volume != volume {
                    self.volume = volume;
                    self.synth.set_volume(self.volume);
                }

                let mut keys = [super::synth::SynthKeyState::Off; 88];
                self.synth.copy_keys(&mut keys);
                super::keyboard::show_keyboard(ui, &mut self.keyboard_state, &keys, &self.midi_write);
            });
        });
    }

    fn update_audio_error(&self, ctx: &egui::Context, error_message: &str) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true), |ui| {
                    ui.label(error_message);
                });
            });
        });
    }
}

impl eframe::App for KeySynthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(error_message) = &self.audio_error {
            self.update_audio_error(ctx, error_message);
        } else {
            self.update_menu(ctx);
            self.update_footer(ctx);
            self.update_central_panel(ctx);
        }
    }
}
