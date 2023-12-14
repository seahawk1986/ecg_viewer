use egui::{global_dark_light_mode_buttons, Context};
use std::future::Future;
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::sample_data::Channel;
use crate::SampleData;

pub struct MonitorApp {
    text_channel: (Sender<String>, Receiver<String>),
    sample_text: String,
    data: Vec<SampleData>,
}

impl Default for MonitorApp {
    fn default() -> Self {
        let sqw = Channel::square_wave(1000, 1000.0, 100_000, None);
        let sin = Channel::sin_wave(1000.0, 100_000, None);
        let wave_data = SampleData::new(
            "Various Wave forms".to_string(),
            vec![sqw, sin],
            1000.0,
            "mV".to_string(),
        );
        let point_data: SampleData = SampleData::new(
            "Point experiments".to_string(),
            vec![Channel::dot_every_n(1000, 1000.0, 100_000, None)],
            1000.0,
            "RR".to_string(),
        );

        Self {
            text_channel: channel(),
            sample_text: "Hier könnte ihre Werbung stehen".into(),
            // dropped_files: vec![],
            data: vec![wave_data, point_data],
        }
    }
}

impl MonitorApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        MonitorApp::default()
    }
}

impl eframe::App for MonitorApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // assign sample text once it comes in
        if let Ok(f) = self.text_channel.1.try_recv() {
            self.sample_text = f;
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                // a simple button opening the dialog
                if ui.button("Add data from file").clicked() {
                    let sender = self.text_channel.0.clone();
                    let task = rfd::AsyncFileDialog::new().pick_file();
                    execute(async move {
                        let file = task.await;
                        if let Some(file) = file {
                            let text = file.read().await;
                            let _ = sender.send(String::from_utf8_lossy(&text).to_string());
                            // TODO: parse file and crate a SampleData Struct from it
                        }
                    });
                }
                ui.separator();
                global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(
                egui::Layout::top_down_justified(egui::Align::Center),
                |ui| {
                    self.data.iter_mut().for_each(|data|{
                        data.plot(ui);
                    });

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::LEFT), |ui| {
                        ui.label(
                            "Double click graph to reset view.\nHold SHIFT to scroll horizontally.\nHold CTRL to zoom in/out.\nDrag with the right mouse button pressed to select a zoom area",
                        );
                    });

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(&self.sample_text);
                    });
                },
            );
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || futures::executor::block_on(f));
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}