use egui::{global_dark_light_mode_buttons, Context, Modifiers};
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::data_structures::{DrawableChannel, SampleBasedChannel, TimeBasedChannel};
use crate::{parse_content, ChannelPlotter};

use std::future::Future;

enum AppState {
    Startup,
    ImportData,
    GraphView,
    LoadingScreen,
}

pub struct MonitorApp {
    text_channel: (Sender<String>, Receiver<String>),
    data_channel: (
        Sender<Vec<SampleBasedChannel>>,
        Receiver<Vec<SampleBasedChannel>>,
    ),
    time_data_channel: (
        Sender<Vec<TimeBasedChannel>>,
        Receiver<Vec<TimeBasedChannel>>,
    ),
    sample_text: String,
    // data: Vec<SampleData>,
    take_screenshot: bool,
    app_state: AppState,
    plotter: ChannelPlotter,
}

impl Default for MonitorApp {
    fn default() -> Self {
        let plotter = ChannelPlotter::new("ECG".to_owned(), vec![]);

        Self {
            text_channel: channel(),
            data_channel: channel(),
            time_data_channel: channel(),
            sample_text: "Hier könnte ihre Werbung stehen".into(),
            take_screenshot: false,
            app_state: AppState::Startup,
            plotter,
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
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                // Check for returned screenshot:
                ui.input(|i| {
                    for event in &i.raw.events {
                        if let egui::Event::Screenshot { image, .. } = event {
                            if self.take_screenshot {
                                println!("we got a screenshot?!");
                                // save the screenshot, if we got one
                                let filename =
                                    format!("screenshot_{}.png", chrono::offset::Local::now());

                                image::save_buffer(
                                    filename,
                                    image.as_raw(),
                                    image.width() as u32,
                                    image.height() as u32,
                                    image::ColorType::Rgba8,
                                )
                                .unwrap();
                                self.take_screenshot = false;
                            }
                        }
                    }
                });
                #[cfg(not(target_arch = "wasm32"))]
                ui.input_mut(|i| {
                    // the wasm backend doesn't have the capability to take screenshots
                    if i.consume_shortcut(&egui::KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        key: egui::Key::F9,
                    }) {
                        self.take_screenshot = true;
                    }
                });
                // a simple button opening the dialog
                if ui.button("Add data from file").clicked() {
                    // let sender = self.text_channel.0.clone();
                    let sample_data_sender = self.data_channel.0.clone();
                    let time_data_sender = self.time_data_channel.0.clone();
                    let task = rfd::AsyncFileDialog::new().pick_files();

                    execute(async move {
                        let file = task.await;
                        if let Some(mut filehandles) = file {
                            while let Some(filehandle) = filehandles.pop() {
                                // workaround because we can't have async closures yet
                                dbg!(&filehandle);
                                let raw_data = filehandle.read().await;
                                let data_channels = parse_content(raw_data);
                                let _ = sample_data_sender.send(data_channels.0);
                                let _ = time_data_sender.send(data_channels.1);
                            }
                        }
                    });
                }
                ui.separator();
                if ui.button("Clear loaded data").clicked() {
                    self.plotter.channels.clear();
                }
                ui.separator();

                #[cfg(not(target_arch = "wasm32"))]
                // the wasm backend doesn't have the capability to take screenshots
                {
                    if ui.button("Screenshot").clicked() {
                        self.take_screenshot = true;
                    }
                    ui.separator();
                }
                global_dark_light_mode_buttons(ui);
            });
        });
        match self.app_state {
            AppState::Startup => {
                self.app_state = AppState::ImportData;
            }
            AppState::ImportData => {
                self.app_state = AppState::LoadingScreen;
            }
            AppState::LoadingScreen => self.app_state = AppState::GraphView,
            AppState::GraphView => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.with_layout(
                        egui::Layout::top_down_justified(egui::Align::Center),
                        |ui| {
                            self.plotter.plot(ui);

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
        // assign sample text once it comes in
        if let Ok(f) = self.text_channel.1.try_recv() {
            self.sample_text = f;
        }
        if let Ok(mut channels) = self.data_channel.1.try_recv() {
            channels.drain(..).for_each(|c| {
                self.plotter
                    .add_channel(Box::new(c) as Box<dyn DrawableChannel>);
            });
        }
        if let Ok(mut channels) = self.time_data_channel.1.try_recv() {
            channels.drain(..).for_each(|c| {
                self.plotter
                    .add_channel(Box::new(c) as Box<dyn DrawableChannel>);
            });
        }

        // request a screenshot if the flag is set
        if self.take_screenshot {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
        }
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
