use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use super::midi_message::{MidiMessage, MidiKeyEvent};

#[derive(Clone)]
struct Inner {
    keys: [u8; Synth::NUM_KEYS],
    midi_connected: bool,
}

#[derive(Clone)]
pub struct Synth {
    inner: Arc<Mutex<Inner>>,
}

impl Synth {
    pub const NUM_KEYS: usize = 88;

    pub fn is_midi_connected(&self) -> bool {
        self.inner.lock().unwrap().midi_connected
    }

    pub fn set_midi_connected(&self, connected: bool) {
        self.inner.lock().unwrap().midi_connected = connected;
    }

    pub fn set_key(&self, key: u8, state: u8) {
        let key = key as usize;
        if key >= Synth::NUM_KEYS { return; }
        let mut inner = self.inner.lock().unwrap();
        inner.keys[key] = state;
    }

    pub fn copy_keys(&self, keys: &mut [u8]) {
        if keys.len() != Self::NUM_KEYS { return; }
        let inner = self.inner.lock().unwrap();
        keys.clone_from_slice(&inner.keys);
    }

    fn run(&self, midi_read: mpsc::Receiver<(u64, MidiMessage)>, frame: egui::Context) {
        loop {
            while let Ok((_stamp, msg)) = midi_read.try_recv() {
                match msg {
                    MidiMessage::PortConnected => {
                        self.set_midi_connected(true);
                        frame.request_repaint();
                    }
                    MidiMessage::PortDisconnected => {
                        self.set_midi_connected(false);
                        frame.request_repaint();
                    }
                    MidiMessage::NoteOn(_, MidiKeyEvent { key, pressure }) => {
                        self.set_key(key, pressure);
                        frame.request_repaint();
                    }
                    MidiMessage::NoteOff(_, MidiKeyEvent { key, .. }) => {
                        self.set_key(key, 0);
                        frame.request_repaint();
                    }
                    _ => {
                        //println!("-> [{:016x}] {:?}", stamp, msg);
                    }
                }
            }
        }
    }

    pub fn start(midi_read: mpsc::Receiver<(u64, MidiMessage)>, frame: egui::Context) -> Self {
        let synth = Synth {
            inner: Arc::new(Mutex::new(Inner {
                keys: [0; 88],
                midi_connected: false,
            })),
        };

        let synth_clone = synth.clone();
        thread::spawn(move || {
            synth_clone.run(midi_read, frame);
        });
        synth
    }
}
