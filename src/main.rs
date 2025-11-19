mod midi_message;
mod midi_reader;
mod midi_ports;
mod synth;
mod synth_voice;
mod keyboard;
mod app;

fn main() -> eframe::Result {
    let viewport = egui::ViewportBuilder::default().with_inner_size([1800.0, 350.0]).with_min_inner_size([640.0, 236.0]);
    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "Key Synth",
        options,
        Box::new(|cc| {
            Ok(Box::new(app::KeySynthApp::new(cc)))
        })
    )
}
