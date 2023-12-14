use chrono::Duration;

use egui::{Color32, Ui};
use egui_plot::{GridMark, Legend, Line, Plot, PlotPoints, Points};

pub enum PlotType {
    Points,
    Line,
}

pub struct SampleData {
    pub name: String,
    pub channels: Vec<Channel>,
    pub samples_per_second: f64,
    pub unit_y: String,
    // TODO: handle time information
}

impl SampleData {
    pub fn new(
        name: String,
        channels: Vec<Channel>,
        samples_per_second: f64,
        unit_y: String,
    ) -> SampleData {
        SampleData {
            name,
            channels,
            samples_per_second,
            unit_y,
        }
    }

    pub fn plot(&mut self, ui: &mut Ui) {
        let unit_label = self.unit_y.to_string();
        Plot::new(self.name.to_string())
            .view_aspect(5.0)
            // .auto_bounds_x()
            .set_margin_fraction(egui::Vec2 { x: 0.1, y: 0.1 })
            .auto_bounds_y()
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
            .x_grid_spacer(|grid_input| {
                /*
                This is a tricky one - we want the classic ecg grid with
                0.04s between the smallest marks, 0.2 s between the medium marks and 1.0 s between the large marks
                but if we zoom out 10 s steps, minute, 5 minute and hour marks - so we go a little crazy
                */

                // generate_marks and fill_marks_between are from the sources of
                // egui_plot: https://librepvz.github.io/librePvZ/src/egui/widgets/plot/mod.rs.html#1634 ff.

                /// Fill in all values between [min, max] which are a multiple of `step_size`
                fn generate_marks(step_sizes: [f64; 3], bounds: (f64, f64)) -> Vec<GridMark> {
                    let mut steps = vec![];
                    fill_marks_between(&mut steps, step_sizes[0], bounds);
                    fill_marks_between(&mut steps, step_sizes[1], bounds);
                    fill_marks_between(&mut steps, step_sizes[2], bounds);
                    steps
                }

                /// Fill in all values between [min, max] which are a multiple of `step_size`
                fn fill_marks_between(
                    out: &mut Vec<GridMark>,
                    step_size: f64,
                    (min, max): (f64, f64),
                ) {
                    assert!(max > min);
                    let first = (min / step_size).ceil() as i64;
                    let last = (max / step_size).ceil() as i64;

                    let marks_iter = (first..last).map(|i| {
                        let value = (i as f64) * step_size;
                        GridMark { value, step_size }
                    });
                    out.extend(marks_iter);
                }

                // now let's generate the grid marks based on the base_step_size
                if grid_input.base_step_size >= 60.0 {
                    generate_marks([60.0, 300.0, 3600.0], grid_input.bounds)
                } else if grid_input.base_step_size >= 10.0 {
                    generate_marks([10.0, 60.0, 300.0], grid_input.bounds)
                } else if grid_input.base_step_size >= 1.0 {
                    generate_marks([1.0, 10.0, 60.0], grid_input.bounds)
                } else if grid_input.base_step_size >= 0.2 {
                    generate_marks([0.2, 1.0, 10.0], grid_input.bounds)
                } else {
                    generate_marks([0.04, 0.2, 1.0], grid_input.bounds)
                }
            })
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
                                .name(channel.name.to_string());
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
                                .name(channel.name.to_string());
                            plot_ui.points(points);
                        }
                    }
                });
            });
    }
}

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
