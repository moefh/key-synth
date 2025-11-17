use std::sync::mpsc;

use super::midi_message::MidiMessage;
use super::midi_reader::{MidiReaderCommand, MidiReaderConfigAcceptedPorts};
use super::synth::Synth;

const DEFAULT_SLEEP_TIME: u64 = 5000;
const DEFAULT_MIDI_PORTS: &[&str] = &[
    // MIDI inputs we'll try to connect to at startup (the actual port
    // name just has to contain the text here to be considered):
    "SMK25-Master"
];

#[allow(dead_code)]
pub struct KeySynthApp {
    midi_write: mpsc::Sender<(u64, MidiMessage)>,
    midi_command: Option<mpsc::Sender<MidiReaderCommand>>,
    midi_in: Option<midir::MidiInput>,
    synth: Synth,
    midi_in_ports: Vec<String>,
    midi_in_ports_refresh_time: Option<std::time::Instant>,
    keyboard_state: super::keyboard::KeyboardState,
}

impl KeySynthApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let (midi_write, midi_read, midi_command, midi_in) = Self::open_midi();

        egui_extras::install_image_loaders(&cc.egui_ctx);
        cc.egui_ctx.set_zoom_factor(1.5);
        KeySynthApp {
            synth: Synth::start(midi_read, cc.egui_ctx.clone()),
            midi_write,
            midi_command,
            midi_in,
            midi_in_ports: Vec::new(),
            midi_in_ports_refresh_time: None,
            keyboard_state: super::keyboard::KeyboardState::new(),
        }
    }

    fn open_midi() -> (mpsc::Sender<(u64, MidiMessage)>, mpsc::Receiver<(u64, MidiMessage)>,
                       Option<mpsc::Sender<MidiReaderCommand>>, Option<midir::MidiInput>) {
        let midi_in = midir::MidiInput::new("MIDI portlist").ok();
        if let Ok(midi) = super::midi_reader::MidiReader::start(DEFAULT_SLEEP_TIME, DEFAULT_MIDI_PORTS) {
            (midi.write, midi.read, Some(midi.command), midi_in)
        } else {
            let (write, read) = mpsc::channel::<(u64, MidiMessage)>();
            (write, read, None, midi_in)
        }
    }

    pub fn close_midi(&self) {
        if let Some(command) = &self.midi_command {
            command.send(MidiReaderCommand::Close).unwrap_or(());
        }
    }

    pub fn select_midi_port(&self, port: String) {
        if let Some(command) = &self.midi_command {
            let cfg = MidiReaderConfigAcceptedPorts { accepted_midi_ports: vec![port] };
            command.send(MidiReaderCommand::ConfigAcceptedPorts(cfg)).unwrap_or(());
        }
    }

    pub fn read_midi_in_ports(&mut self) -> &[String] {
        // if we read the port list less than 10 seconds ago,
        // return the last list
        if let Some(instant) = self.midi_in_ports_refresh_time && instant.elapsed().as_secs() <= 10 {
            return &self.midi_in_ports;
        }

        // refresh the port list
        if let Some(midi_in) = &self.midi_in {
            self.midi_in_ports.clear();
            for port in midi_in.ports() {
                if let Ok(p) = midi_in.port_name(&port) {
                    self.midi_in_ports.push(p);
                }
            }
            self.midi_in_ports_refresh_time = Some(std::time::Instant::now());
        }
        &self.midi_in_ports
    }

    fn update_menu(&mut self, ctx: &egui::Context) {
        let mut select_midi_port = None;
        egui::TopBottomPanel::top("main_menu").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Synth", |ui| {
                    if ui.button("Quit").clicked() {
                        self.close_midi();
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                if self.midi_command.is_some() && self.midi_in.is_some() {
                    ui.menu_button("Midi In", |ui| {
                        for port in self.read_midi_in_ports() {
                            if ui.button(port).clicked() {
                                select_midi_port = Some(port.to_owned());
                            }
                        }
                    });
                }
            });
        });
        if let Some(port) = select_midi_port {
            self.select_midi_port(port);
        }
    }

    fn update_footer(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            //ui.add_space(5.0);
            if self.synth.is_midi_connected() {
                ui.label("MIDI input connected");
            } else {
                ui.label("MIDI input not connected");
            }
        });
    }

    fn update_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut keys = [0; 88];
            self.synth.copy_keys(&mut keys);
            super::keyboard::show_keyboard(ui, &mut self.keyboard_state, &keys);
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
