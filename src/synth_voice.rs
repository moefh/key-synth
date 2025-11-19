#[derive(Clone)]
pub struct SynthVoice {
    pub active: bool,
    pub stopping: bool,
    pub key: u8,
    pub freq: f32,
    pub volume: f32,
    pub tick: f32,
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
        self.freq = Self::get_midi_note_frequency(key as i32);
        self.volume = pressure as f32 / 127.0;
        self.tick = 0.0;
    }

    pub fn stop(&mut self) {
        self.stopping = true;
    }

    pub fn gen_samples(&mut self, data: &mut [i16]) {
        let mut t = self.tick;
        let freq = self.freq;
        let mut volume = self.volume;
        let stopping = self.stopping;
        let vol_delta = if stopping { -volume / data.len() as f32 } else { 0.0 };
        for spl in data.iter_mut() {
            let val = (t * std::f32::consts::TAU / Self::SAMPLE_RATE as f32 * freq).sin() * 8000.0 * volume;
            *spl += val.clamp(i16::MIN as f32, i16::MAX as f32).round() as i16;
            t += 1.0;
            volume += vol_delta;
        }
        self.tick = t;
        if stopping {
            self.active = false;
        } else {
            self.volume *= 0.99;
        }
    }
}
