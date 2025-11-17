#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MidiKeyEvent {
    pub key: u8,
    pub pressure: u8,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MidiControlEvent {
    pub control: u8,
    pub value: u8,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MidiProgramChangeEvent {
    pub number: u8,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MidiAftertouchEvent {
    pub pressure: u8,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MidiPitchEvent {
    pub wheel: u16,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MidiSysExEvent {
    pub data: [u8; 2],
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum MidiMessage {
    PortConnected,
    PortDisconnected,
    Invalid,
    NoteOn(u8, MidiKeyEvent),
    NoteOff(u8, MidiKeyEvent),
    PolyAfertouch(u8, MidiKeyEvent),
    ControlChange(u8, MidiControlEvent),
    ProgramChange(u8, MidiProgramChangeEvent),
    ChannelAftertouch(u8, MidiAftertouchEvent),
    PitchWheel(u8, MidiPitchEvent),
    SysEx(u8, MidiSysExEvent),
}

impl MidiMessage {
    pub fn decode(data: &[u8]) -> Self {
        let chan = (data[0] & 0x0f) + 1;
        match data[0] & 0xf0 {
            0x80 => if data.len() >= 3 {
                MidiMessage::NoteOff(chan, MidiKeyEvent { key: data[1], pressure: data[2] })
            } else {
                MidiMessage::Invalid
            }

            0x90 => if data.len() >= 3 {
                MidiMessage::NoteOn(chan, MidiKeyEvent { key: data[1], pressure: data[2] })
            } else {
                MidiMessage::Invalid
            }

            0xA0 => if data.len() >= 3 {
                MidiMessage::PolyAfertouch(chan, MidiKeyEvent { key: data[1], pressure: data[2] })
            } else {
                MidiMessage::Invalid
            }

            0xB0 => if data.len() >= 3 {
                MidiMessage::ControlChange(chan, MidiControlEvent { control: data[1], value: data[2] })
            } else {
                MidiMessage::Invalid
            }

            0xC0 => if data.len() >= 2 {
                MidiMessage::ProgramChange(chan, MidiProgramChangeEvent { number: data[1] })
            } else {
                MidiMessage::Invalid
            }

            0xD0 => if data.len() >= 2 {
                MidiMessage::ChannelAftertouch(chan, MidiAftertouchEvent { pressure: data[1] })
            } else {
                MidiMessage::Invalid
            }

            0xE0 => if data.len() >= 3 {
                MidiMessage::PitchWheel(chan, MidiPitchEvent { wheel: ((data[2] as u16) << 8) | (data[1] as u16) })
            } else {
                MidiMessage::Invalid
            }

            0xF0 => MidiMessage::SysEx(chan, MidiSysExEvent { data: [data[1], data[2]] }),

            _ => MidiMessage::Invalid,
        }
    }
}
