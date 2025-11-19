pub struct MidiPorts {
    midi_in: midir::MidiInput,
    port_names: Vec<String>,
    refresh_time: Option<std::time::Instant>,
}

impl MidiPorts {
    pub fn open() -> Option<Self> {
        let midi_in = midir::MidiInput::new("MIDI portlist").ok()?;
        Some(MidiPorts {
            midi_in,
            port_names: Vec::new(),
            refresh_time: None,
        })
    }

    pub fn read_port_names(&mut self) -> &[String] {
        // if we read the port list less than 10 seconds ago,
        // return the last list
        if let Some(instant) = self.refresh_time && instant.elapsed().as_secs() <= 10 {
            return &self.port_names;
        }

        // refresh the port list
        self.port_names.clear();
        for port in self.midi_in.ports() {
            if let Ok(p) = self.midi_in.port_name(&port) {
                self.port_names.push(p);
            }
        }
        self.refresh_time = Some(std::time::Instant::now());
        &self.port_names
    }
}
