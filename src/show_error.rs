
pub struct ShowErrorApp {
    message: String,
}

impl ShowErrorApp {
    pub fn new(cc: &eframe::CreationContext, message: String) -> Self {
        cc.egui_ctx.set_zoom_factor(1.5);
        ShowErrorApp {
            message,
        }
    }
}

impl eframe::App for ShowErrorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Error Initializing Sound");
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true), |ui| {
                    ui.label(&self.message);
                });
            });
        });
    }
}
