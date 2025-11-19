use std::sync::mpsc;

use super::midi_message::MidiMessage;
use super::midi_reader::{MidiReaderCommand, MidiReaderConfigAcceptedPorts};
use super::synth::SynthKeyboard;

const DEFAULT_SLEEP_TIME: u64 = 5000;
const DEFAULT_MIDI_PORTS: &[&str] = &[
    // MIDI inputs we'll try to connect to at startup (the actual port
    // name just has to contain the text here to be considered):
    "SMK25",
];

pub struct KeySynthApp {
    midi_write: mpsc::Sender<MidiMessage>,
    reader_command: Option<mpsc::Sender<MidiReaderCommand>>,
    midi_ports: Option<super::midi_ports::MidiPorts>,
    synth: SynthKeyboard,
    keyboard_state: super::keyboard::KeyboardState,
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

        egui_extras::install_image_loaders(&cc.egui_ctx);
        //cc.egui_ctx.set_theme(egui::ThemePreference::Light);
        cc.egui_ctx.set_zoom_factor(1.5);
        KeySynthApp {
            synth,
            midi_write,
            reader_command,
            midi_ports: super::midi_ports::MidiPorts::open(),
            keyboard_state: super::keyboard::KeyboardState::new(),
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
            let mut keys = [super::synth::SynthKeyState::Off; 88];
            self.synth.copy_keys(&mut keys);
            super::keyboard::show_keyboard(ui, &mut self.keyboard_state, &keys, &self.midi_write);
        });
    }
}

impl eframe::App for KeySynthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_menu(ctx);
        self.update_footer(ctx);
        self.update_central_panel(ctx);
    }
}
