use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use super::midi_message::{MidiMessage, MidiKeyEvent};
use super::synth_voice::{SynthVoice, SynthInstrument};

#[derive(Clone, Copy, Debug)]
pub struct SynthVoiceIndex(usize);

#[derive(Clone, Copy, Debug)]
pub enum SynthKeyState {
    Off,
    Playing(SynthVoiceIndex),
    VoiceStolen,
}

#[allow(dead_code)]
struct CpalSoundOutput {
    host: cpal::Host,
    device: cpal::Device,
    stream: cpal::Stream,
}

//#[derive(Clone)]
struct SynthInner {
    voices: [SynthVoice; SynthInner::MAX_VOICES],
    keys: [SynthKeyState; SynthInner::NUM_KEYS],
    next_voice: usize,
    midi_connected: bool,
    volume: f32,
}

impl SynthInner {
    pub const MAX_VOICES: usize = 8;
    pub const NUM_KEYS: usize = 88;

    fn new() -> Self {
        SynthInner {
            voices: [SynthVoice::EMPTY; SynthInner::MAX_VOICES],
            keys: [SynthKeyState::Off; Self::NUM_KEYS],
            next_voice: 0,
            midi_connected: false,
            volume: 0.7,
        }
    }

    fn get_new_voice(&mut self) -> usize {
        // if the next voice is available, use it
        if ! self.voices[self.next_voice].active {
            let voice_index = self.next_voice;
            self.next_voice = (self.next_voice + 1) % Self::MAX_VOICES;
            return voice_index;
        }

        // check if any other voice is available; if not, use the next voice anyway
        let mut voice_index = self.next_voice;
        for _ in 0..Self::MAX_VOICES {
            voice_index = (voice_index + 1) % Self::MAX_VOICES;
            if ! self.voices[voice_index].active {
                break;
            }
        }
        self.next_voice = (voice_index + 1) % Self::MAX_VOICES;
        voice_index
    }

    fn play_key(&mut self, key: u8, pressure: u8) {
        let key_index = key as usize;

        // if this key is already playing, just start it again
        if let SynthKeyState::Playing(SynthVoiceIndex(voice_index)) = self.keys[key_index] {
            self.voices[voice_index].start(key, pressure, self.volume);
            return;
        }

        // get a new voice to play
        let voice_index = self.get_new_voice();

        // If the voice was playing a key, mark the key as having the
        // voice stolen.  Sadly, this will produce an audible "pop" as
        // the stolen voice gets cutoff abruptly.
        if self.voices[voice_index].active {
            let stolen_key = self.voices[voice_index].key as usize;
            self.keys[stolen_key] = SynthKeyState::VoiceStolen;
        }

        // start playing the new voice
        self.voices[voice_index].start(key, pressure, self.volume);
        self.keys[key_index] = SynthKeyState::Playing(SynthVoiceIndex(voice_index));
    }

    fn stop_key(&mut self, key: u8) {
        let key_index = key as usize;
        if let SynthKeyState::Playing(SynthVoiceIndex(voice_index)) = self.keys[key_index] {
            self.voices[voice_index].stop();
        }
        self.keys[key_index] = SynthKeyState::Off;
    }

    fn set_instrument(&mut self, instrument: SynthInstrument) {
        for voice in self.voices.iter_mut() {
            voice.set_instrument(instrument);
        }
    }
}

#[derive(Clone)]
pub struct SynthKeyboard {
    inner: Arc<Mutex<SynthInner>>,
}

impl SynthKeyboard {
    #[allow(dead_code)]
    pub const MAX_VOICES: usize = SynthInner::MAX_VOICES;
    pub const NUM_KEYS: usize = SynthInner::NUM_KEYS;

    pub fn is_midi_connected(&self) -> bool {
        self.inner.lock().unwrap().midi_connected
    }

    pub fn set_midi_connected(&self, connected: bool) {
        self.inner.lock().unwrap().midi_connected = connected;
    }

    pub fn play_key(&self, key: u8, pressure: u8) {
        let key_index = key as usize;
        if key_index >= Self::NUM_KEYS { return; }
        let mut inner = self.inner.lock().unwrap();
        inner.play_key(key, pressure);
    }

    pub fn stop_key(&self, key: u8) {
        let key_index = key as usize;
        if key_index >= Self::NUM_KEYS { return; }
        let mut inner = self.inner.lock().unwrap();
        inner.stop_key(key);
    }

    pub fn copy_keys(&self, keys: &mut [SynthKeyState]) {
        if keys.len() != Self::NUM_KEYS { return; }
        let inner = self.inner.lock().unwrap();
        keys.clone_from_slice(&inner.keys);
    }

    pub fn set_instrument(&self, instrument: SynthInstrument) {
        let mut inner = self.inner.lock().unwrap();
        inner.set_instrument(instrument);
    }

    pub fn get_volume(&self) -> f32 {
        let inner = self.inner.lock().unwrap();
        inner.volume
    }

    pub fn set_volume(&self, volume: f32) {
        let mut inner = self.inner.lock().unwrap();
        inner.volume = volume;
    }

    fn open_sound_out(&self) -> Option<CpalSoundOutput> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let supported_config_range = device.supported_output_configs().ok()?.find(|range| {
            matches!(range.sample_format(), cpal::SampleFormat::I16) &&
                range.channels() == 1 &&
                range.min_sample_rate().0 <= SynthVoice::SAMPLE_RATE &&
                range.max_sample_rate().0 >= SynthVoice::SAMPLE_RATE &&
                matches!(range.buffer_size(), cpal::SupportedBufferSize::Range{
                    min: 0..=SynthVoice::BUFFER_SIZE,
                    max:SynthVoice::BUFFER_SIZE..=u32::MAX
                })
        });
        let mut config = supported_config_range?.try_with_sample_rate(cpal::SampleRate(SynthVoice::SAMPLE_RATE))?.config();
        config.buffer_size = cpal::BufferSize::Fixed(SynthVoice::BUFFER_SIZE);

        let synth_inner = self.inner.clone();
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                for spl in data.iter_mut() {
                    *spl = 0;
                }
                let mut inner = synth_inner.lock().unwrap();
                for voice in inner.voices.iter_mut() {
                    if voice.active {
                        voice.gen_samples(data);
                    }
                }
            },
            move |err| { println!("CPAL error: {}", err); },
            None).ok()?;
        stream.play().ok()?;

        Some(CpalSoundOutput {
            host,
            device,
            stream,
        })
    }

    fn run(&self, midi_read: mpsc::Receiver<MidiMessage>, egui_ctx: egui::Context) {
        let _sound_out = self.open_sound_out();

        loop {
            while let Ok(msg) = midi_read.try_recv() {
                match msg {
                    MidiMessage::PortConnected => {
                        self.set_midi_connected(true);
                        egui_ctx.request_repaint();
                    }
                    MidiMessage::PortDisconnected => {
                        self.set_midi_connected(false);
                        egui_ctx.request_repaint();
                    }
                    MidiMessage::NoteOn(_, MidiKeyEvent { key, pressure }) => {
                        self.play_key(key, pressure);
                        egui_ctx.request_repaint();
                    }
                    MidiMessage::NoteOff(_, MidiKeyEvent { key, .. }) => {
                        self.stop_key(key);
                        egui_ctx.request_repaint();
                    }
                    _ => {
                        //println!("-> [{:016x}] {:?}", stamp, msg);
                    }
                }
            }
        }
    }

    pub fn start(midi_read: mpsc::Receiver<MidiMessage>, egui_ctx: egui::Context) -> Self {
        let sound_writer = SynthKeyboard {
            inner: Arc::new(Mutex::new(SynthInner::new())),
        };

        let sw = sound_writer.clone();
        thread::spawn(move || {
            sw.run(midi_read, egui_ctx);
        });
        sound_writer
    }
}
