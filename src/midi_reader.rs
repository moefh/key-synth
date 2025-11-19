use std::result::Result;
use std::error::Error;
use std::sync::mpsc;
use midir::{MidiInput, MidiInputPort};

use super::midi_message::MidiMessage;

pub struct MidiReaderConfigAcceptedPorts {
    pub accepted_midi_ports: Vec<String>,
}

#[allow(dead_code)]
pub struct MidiReaderConfigSleepTime {
    pub sleep_time_millis: u64,
}

#[allow(dead_code)]
pub enum MidiReaderCommand {
    Close,
    ConfigAcceptedPorts(MidiReaderConfigAcceptedPorts),
    ConfigSleepTime(MidiReaderConfigSleepTime),
}

struct MidiConnector {
    accepted_midi_ports: Vec<String>,
    sleep_time_millis: u64,
    midi_check: MidiInput,
    command_receiver: mpsc::Receiver<MidiReaderCommand>,
    midi_sender: mpsc::Sender<MidiMessage>,
    connected_port_name: Option<String>,
}

struct MidiReaderData {
    midi_in: MidiInput,
    stop: bool,
}

impl MidiConnector {
    fn has_connected_midi_in_port(&self) -> bool {
        if let Some(connected_port_name) = &self.connected_port_name {
            for port in self.midi_check.ports() {
                let port_name = match self.midi_check.port_name(&port) {
                    Ok(p) => p,
                    Err(_) => { return false; }
                };
                if port_name == *connected_port_name {
                    return true;
                }
            }
        }
        false
    }

    fn select_midi_in_port(&self, midi_in: &MidiInput) -> Result<(MidiInputPort, String), Box<dyn Error>> {
        for port in midi_in.ports() {
            let port_name = &midi_in.port_name(&port)?;
            if self.accepted_midi_ports.iter().any(|a| port_name.contains(a)) {
                return Ok((port, port_name.clone()));
            }
        }
        Err("no suitable port found".into())
    }

    fn run_step(&mut self, data: MidiReaderData) -> MidiReaderData {
        let sleep_time = std::time::Duration::from_millis(self.sleep_time_millis);

        // select input port
        let (in_port, in_port_name) = loop {
            match self.select_midi_in_port(&data.midi_in) {
                Ok(v) => break v,
                Err(_) => {
                    // error selecting port, sleep and check for commands
                    match self.command_receiver.recv_timeout(sleep_time) {
                        Ok(MidiReaderCommand::Close) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                            return MidiReaderData {
                                midi_in: data.midi_in,
                                stop: true,     // stop trying to connect, exit midi reader
                            };
                        }

                        Ok(MidiReaderCommand::ConfigAcceptedPorts(cfg)) => {
                            self.accepted_midi_ports = cfg.accepted_midi_ports;
                        }

                        Ok(MidiReaderCommand::ConfigSleepTime(cfg)) => {
                            self.sleep_time_millis = cfg.sleep_time_millis;
                        }

                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            // keep trying to select port
                        }
                    }
                }
            }
        };

        // connect to selected port
        let connect_result = data.midi_in.connect(
            &in_port,
            "midir-read-input",
            move |_stamp, message, midi_sender| {
                //println!("data: {:x?}", message);
                let midi_message = MidiMessage::decode(message);
                if let Err(e) = midi_sender.send(midi_message) {
                    println!("ERROR sending MIDI message: {}", e);
                }
            },
            self.midi_sender.clone()
        );
        let midi_in_connection = match connect_result {
            Err(e) => {
                self.connected_port_name = None;
                std::thread::sleep(sleep_time);
                return MidiReaderData {
                    midi_in: e.into_inner(),
                    stop: false,
                };
            }
            Ok(conn) => {
                self.connected_port_name = Some(in_port_name);
                self.midi_sender.send(MidiMessage::PortConnected).unwrap_or(());
                conn
            }
        };

        // read commands and monitor the input ports (to check if the selected input port still exists)
        loop {
            match self.command_receiver.recv_timeout(sleep_time) {
                Ok(MidiReaderCommand::Close) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // disconnect and exit midi reader
                    self.midi_sender.send(MidiMessage::PortDisconnected).unwrap_or(());
                    self.connected_port_name = None;
                    let (midi_in, _) = midi_in_connection.close();
                    return MidiReaderData {
                        midi_in,
                        stop: true,
                    };
                }

                Ok(MidiReaderCommand::ConfigAcceptedPorts(cfg)) => {
                    // change configuration and disconnect/reconnect
                    self.accepted_midi_ports = cfg.accepted_midi_ports;
                    self.midi_sender.send(MidiMessage::PortDisconnected).unwrap_or(());
                    self.connected_port_name = None;
                    let (midi_in, _) = midi_in_connection.close();
                    return MidiReaderData {
                        midi_in,
                        stop: false,
                    };
                }

                Ok(MidiReaderCommand::ConfigSleepTime(cfg)) => {
                    self.sleep_time_millis = cfg.sleep_time_millis;  // keep connection going
                }

                Err(mpsc::RecvTimeoutError::Timeout) => {}           // keep connection going
            }

            // check if the connection's MIDI IN still exists
            if ! self.has_connected_midi_in_port() {
                self.midi_sender.send(MidiMessage::PortDisconnected).unwrap_or(());
                self.connected_port_name = None;
                let (midi_in, _) = midi_in_connection.close();
                return MidiReaderData {
                    midi_in,
                    stop: false,
                };
            }
        }
    }

    fn run(&mut self, midi_in: MidiInput) {
        let mut data = MidiReaderData {
            midi_in,
            stop: false,
        };
        while ! data.stop {
            data = self.run_step(data);
        }
    }
}

pub fn start(sleep_time_millis: u64, accepted_midi_ports: &[&str], midi_sender: mpsc::Sender<MidiMessage>)
             -> Result<mpsc::Sender<MidiReaderCommand>, Box<dyn Error>> {
    let midi_check = MidiInput::new("MIDI check")?;
    let midi_in = MidiInput::new("MIDI in")?;
    let (command_sender, command_receiver) = mpsc::channel::<MidiReaderCommand>();

    let mut connector = MidiConnector {
        sleep_time_millis,
        accepted_midi_ports: accepted_midi_ports.iter().map(|s| (*s).to_owned()).collect(),
        midi_check,
        midi_sender,
        command_receiver,
        connected_port_name: None,
    };
    std::thread::spawn(move || {
        connector.run(midi_in);
    });

    Ok(command_sender)
}
