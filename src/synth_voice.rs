#[derive(Clone, Copy)]
pub struct SynthInstrumentOvertone {
    frequency: f32,
    loudness: f32,
}

#[derive(Clone, Copy)]
pub struct SynthInstrument {
    pub overtones: [SynthInstrumentOvertone; SynthInstrument::NUM_OVERTONES],
    pub decay: f32,
}

impl SynthInstrument {
    const NUM_OVERTONES: usize = 5;
    pub const PIANO: Self = SynthInstrument {
        decay: 0.99,
        overtones: [
            SynthInstrumentOvertone { frequency: 1.00, loudness: 1.0 },
            SynthInstrumentOvertone { frequency: 2.00, loudness: 0.5 },
            SynthInstrumentOvertone { frequency: 3.00, loudness: 0.8 },
            SynthInstrumentOvertone { frequency: 4.00, loudness: 0.1 },
            SynthInstrumentOvertone { frequency: 5.00, loudness: 0.3 },
        ]
    };
    pub const VIBRAPHONE: Self = SynthInstrument {
        decay: 0.98,
        overtones: [
            SynthInstrumentOvertone { frequency: 1.00, loudness: 0.8 },
            SynthInstrumentOvertone { frequency: 2.00, loudness: 0.0 },
            SynthInstrumentOvertone { frequency: 3.00, loudness: 0.0 },
            SynthInstrumentOvertone { frequency: 4.00, loudness: 0.8 },
            SynthInstrumentOvertone { frequency: 5.00, loudness: 0.0 },
        ]
    };
    pub const BELL: Self = SynthInstrument {
        decay: 0.99,
        overtones: [
            SynthInstrumentOvertone { frequency: 1.0, loudness: 1.0 },
            SynthInstrumentOvertone { frequency: 2.2, loudness: 0.6 },
            SynthInstrumentOvertone { frequency: 3.3, loudness: 0.9 },
            SynthInstrumentOvertone { frequency: 4.4, loudness: 0.1 },
            SynthInstrumentOvertone { frequency: 5.5, loudness: 0.3 },
        ]
    };
}

#[derive(Clone, Copy)]
pub struct SynthVoice {
    pub active: bool,
    pub stopping: bool,
    pub key: u8,
    pub freq: f32,
    pub volume: f32,
    pub tick: f32,
    pub instrument: SynthInstrument,
    overtones: [(f32, f32); SynthInstrument::NUM_OVERTONES],
}

impl SynthVoice {
    pub const SAMPLE_RATE: u32 = 48000;
    pub const BUFFER_SIZE: u32 = 1024;
    pub const EMPTY: SynthVoice = SynthVoice {
        active: false,
        stopping: false,
        key: 0,
        freq: 0.0,
        volume: 0.0,
        tick: 0.0,
        instrument: SynthInstrument::PIANO,
        overtones: [(0.0, 0.0); SynthInstrument::NUM_OVERTONES],
    };

    fn get_midi_note_frequency(note: i32) -> f32 {
        // We use standard A440 with A4 = general midi note 69, so the
        // formula for the note frequency is:
        //
        //    f_note = 440 * 2^((note - 69) / 12)
        //
        440.0 * 2.0_f32.powf((note - 69) as f32 / 12.0)
    }

    pub fn start(&mut self, key: u8, pressure: u8) {
        self.key = key;
        self.active = true;
        self.stopping = false;
        self.tick = 0.0;
        self.volume = pressure as f32 / 127.0;
        self.freq = Self::get_midi_note_frequency(key as i32);
        self.update_overtones();
    }

    pub fn stop(&mut self) {
        self.stopping = true;
    }

    fn update_overtones(&mut self) {
        for (i, overtone) in self.overtones.iter_mut().enumerate() {
            (*overtone).0 = self.instrument.overtones[i].frequency * self.freq;
            (*overtone).1 = self.instrument.overtones[i].loudness;
        }
    }

    pub fn set_instrument(&mut self, instrument: SynthInstrument) {
        self.instrument = instrument;
        self.update_overtones();
    }

    pub fn gen_samples(&mut self, data: &mut [i16]) {
        let mut t = self.tick;
        //let freq = self.freq;
        let mut volume = self.volume;
        let stopping = self.stopping;
        let vol_delta = if stopping { -volume / data.len() as f32 } else { 0.0 };
        let overtones = &self.overtones;
        for spl in data.iter_mut() {
            let mut val = 0.0;
            for (freq, mult) in overtones {
                val += (t * std::f32::consts::TAU / Self::SAMPLE_RATE as f32 * freq).sin() * mult * 3000.0 * volume;
            }
            *spl = (*spl).saturating_add(val.clamp(i16::MIN as f32, i16::MAX as f32).round() as i16);
            t += 1.0;
            volume += vol_delta;
        }
        self.tick = t;
        if stopping {
            self.active = false;
        } else {
            self.volume *= self.instrument.decay;
        }
    }
}
