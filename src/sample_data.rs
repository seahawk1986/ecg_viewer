use chrono::Duration;

use egui::{Color32, Ui};
use egui_plot::{Legend, Line, Plot, PlotPoints, Points};

use crate::grid_helper;
use grid_helper::ecg_grid_spacer;

#[derive(Clone, Debug)]
pub enum PlotType {
    Points,
    Line,
}

#[derive(Clone, Debug)]
pub struct SampleData {
    pub name: String,
    pub channels: Vec<Channel>,
    pub unit_y: String,
    // TODO: handle time information
}

impl SampleData {
    pub fn new(name: String, channels: Vec<Channel>, unit_y: String) -> SampleData {
        SampleData {
            name,
            channels,
            unit_y,
        }
    }

    pub fn add_channel(&mut self, channel: Channel) {
        self.channels.push(channel);
    }

    pub fn plot(&mut self, ui: &mut Ui) {
        let unit_label = self.unit_y.to_string();
        Plot::new(self.name.to_string())
            // .view_aspect(5.0)
            // .auto_bounds_x()
            .set_margin_fraction(egui::Vec2 { x: 0.1, y: 0.1 })
            .auto_bounds_y()
            .auto_bounds_x()
            // .custom_y_axes(
            //     self.channels
            //         .iter()
            //         .map(|c| AxisHints::default().label(c.name.to_owned()))
            //         .collect(),
            // )
            // .center_y_axis(true)
            .label_formatter(move |name, value| {
                let time_pos = Duration::nanoseconds((value.x * 1E9) as i64);
                let hours = (time_pos.num_seconds() / 60) / 60;
                let minutes = (time_pos.num_seconds() / 60) % 60;
                let seconds = time_pos.num_seconds() % 60;
                if !name.is_empty() {
                    format!(
                        "{}\n({:.2}:{:.2}) {}\n{:02}:{:02}:{:02}.{:.3}",
                        name,
                        value.x,
                        value.y,
                        unit_label,
                        hours,
                        minutes,
                        seconds,
                        time_pos.num_milliseconds() % 1000
                    )
                } else {
                    "".to_owned()
                    //format!("{}", format_duration(time_pos))
                }
            })
            .legend(Legend::default().position(egui_plot::Corner::LeftBottom))
            .link_axis("ecg", true, false)
            .x_grid_spacer(ecg_grid_spacer)
            .x_axis_label(self.name.to_string())
            .y_axis_label(self.unit_y.to_string())
            // .clamp_grid(true)
            .show(ui, |plot_ui| {
                self.channels.iter().for_each(|channel| {
                    match channel.plot_type {
                        // TODO get displayed bounds - is this a performance optimization?
                        PlotType::Line => {
                            let plot_points: PlotPoints = channel
                                .data
                                .iter()
                                .enumerate()
                                .map(|(idx, y)| {
                                    [
                                        idx as f64 / channel.samples_per_second,
                                        y * channel.scaling_factor,
                                    ]
                                })
                                .collect();
                            let line = Line::new(plot_points)
                                .width(2.0)
                                .color(channel.color)
                                .name(channel.name.to_owned());
                            plot_ui.line(line);
                        }
                        PlotType::Points => {
                            let plot_points: PlotPoints = channel
                                .data
                                .iter()
                                .enumerate()
                                .filter(|(_idx, v)| **v == 0.0f64)
                                .map(|(idx, _y)| {
                                    [
                                        idx as f64 / channel.samples_per_second,
                                        1.0 * channel.scaling_factor,
                                    ]
                                })
                                .collect();
                            let points = Points::new(plot_points)
                                .color(channel.color)
                                .radius(5.0)
                                .stems(0.0) // draw a line to 0
                                .name(channel.name.to_owned());
                            plot_ui.points(points);
                        }
                    }
                });
            });
    }
}

#[derive(Clone, Debug)]
pub struct Channel {
    name: String,
    data: Vec<f64>,
    samples_per_second: f64,
    scaling_factor: f64,
    plot_type: PlotType,
    color: Color32,
}

impl Channel {
    pub fn new(
        name: String,
        data: Vec<f64>,
        samples_per_second: f64,
        scaling_factor: f64,
        plot_type: PlotType,
        color: Option<Color32>,
    ) -> Channel {
        Channel {
            name,
            data,
            samples_per_second,
            scaling_factor,
            plot_type,
            color: color.unwrap_or(Color32::TRANSPARENT),
        }
    }

    pub fn square_wave(
        switch_every_n_samples: i32,
        samples_per_second: f64,
        n_samples: i32,
        color: Option<Color32>,
    ) -> Channel {
        let mut signal = true;
        let square: Vec<f64> = (0..n_samples)
            .map(|n| {
                if n % switch_every_n_samples == 0 {
                    signal = !signal
                };
                signal.into()
            })
            .collect();
        Channel::new(
            format!(
                "square wave, switch every {} samples",
                switch_every_n_samples
            ),
            square,
            samples_per_second,
            1.0,
            PlotType::Line,
            color,
        )
    }

    pub fn sin_wave(samples_per_second: f64, n_samples: usize, color: Option<Color32>) -> Channel {
        let sin: Vec<f64> = (0..n_samples)
            .map(|i| {
                let x = i as f64 * 0.01;
                x.sin()
            })
            .collect();
        Channel::new(
            "sin wave".to_string(),
            sin,
            samples_per_second,
            1.0,
            PlotType::Line,
            color,
        )
    }

    pub fn dot_every_n(
        dot_every_n_samples: usize,
        samples_per_second: f64,
        n_samples: usize,
        color: Option<Color32>,
    ) -> Channel {
        let dots: Vec<f64> = (0..=n_samples)
            .map(|i| (i % dot_every_n_samples) as f64)
            .collect();
        Channel::new(
            "dots".to_string(),
            dots,
            samples_per_second,
            1.0,
            PlotType::Points,
            color,
        )
    }

    pub fn get_slice(&mut self, start: Option<usize>, end: Option<usize>) -> &[f64] {
        let start = start.unwrap_or(0);
        let end = end.unwrap_or(self.data.len());
        &self.data[start..end]
    }
}
