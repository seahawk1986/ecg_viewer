use core::f64;
use std::collections::HashMap;

use chrono::{Duration, NaiveDate, NaiveDateTime};
use csv::StringRecord;
use snafu::prelude::*;

use egui::{Color32, Ui};
use egui_plot::{Legend, Line, Plot, PlotPoints, PlotUi, Points};

use crate::grid_helper;
use grid_helper::ecg_grid_spacer;

#[derive(Clone, Debug)]
pub enum PlotType {
    Points,
    Line,
}

// #[derive(Clone, Debug)]
// pub enum ChannelType {
//     SampleBasedChannel,
//     TimeBasedChannel,
// }

pub enum Filetype {
    PolarECG,
    PolarACC,
    PolarHR,
    PolarRR,
    Unknown,
}

pub trait DrawableChannel {
    fn points_to_draw(&mut self, start_pos: f64, end_pos: f64) -> PlotPoints;
    fn draw(&mut self, plot_ui: &mut PlotUi, start_pos: f64, end_pos: f64);
    fn show_settings(&mut self);
    fn get_name(&mut self) -> String;
    fn get_unit(&mut self) -> String {
        "mV".to_owned()
    }
}

#[derive(Debug, Snafu)]
pub enum ParserError {
    #[snafu(display("Invalid file content"))]
    ContentError,
    #[snafu(display("Data Error: {data_str}"))]
    DataConversionError { data_str: String },
}

// #[derive(Clone, Debug)]
pub struct ChannelPlotter {
    pub name: String,
    pub channels: Vec<Box<dyn DrawableChannel>>,
}

impl ChannelPlotter {
    pub fn new(name: String, channels: Vec<Box<dyn DrawableChannel>>) -> ChannelPlotter {
        ChannelPlotter { name, channels }
    }

    pub fn add_channel(&mut self, channel: Box<dyn DrawableChannel>) {
        self.channels.push(channel);
    }

    pub fn plot(&mut self, ui: &mut Ui) {
        let unit_label = "mV".to_owned();
        let _unit_labels: HashMap<String, String> = self
            .channels
            .iter_mut()
            .map(|c| {
                let channel = c.as_mut();
                (channel.get_name(), channel.get_unit())
            })
            .collect();
        Plot::new(self.name.to_string())
            // .view_aspect(5.0)
            // .data_aspect(1.0)
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
            .x_axis_label("Time [s]".to_owned())
            .y_axis_label("mV".to_owned()) // TODO: respect the individual's channels units
            // .clamp_grid(true)
            .show(ui, |plot_ui| {
                self.channels.iter_mut().for_each(|channel| {
                    channel
                        .as_mut()
                        .draw(plot_ui, f64::NEG_INFINITY, f64::INFINITY);
                });
            });
    }
}

#[derive(Clone, Debug)]
pub struct TimeBasedChannel {
    name: String,
    data: Vec<(NaiveDateTime, f64)>,
    scaling_factor: f64,
    plot_type: PlotType,
    unit: String,
    color: Color32,
}

impl TimeBasedChannel {
    fn new(
        name: String,
        data: Vec<(NaiveDateTime, f64)>,
        scaling_factor: f64,
        plot_type: PlotType,
        unit: String,
        color: Option<Color32>,
    ) -> TimeBasedChannel {
        let color = color.unwrap_or(Color32::TRANSPARENT);
        TimeBasedChannel {
            name,
            data,
            scaling_factor,
            plot_type,
            color,
            unit,
        }
    }

    pub fn parse_polar_data(
        data: String,
        file_type: Filetype,
        n_records: usize,
    ) -> Result<Vec<TimeBasedChannel>, ParserError> {
        // create the csv reader
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b';')
            .from_reader(data.as_bytes());

        // we return one channel per data channel
        let mut channels: Vec<Vec<(NaiveDateTime, f64)>> = vec![];

        // this is the first timestamp we get from the data
        let mut headers: Vec<String> = vec![];
        {
            rdr.headers()
                .expect("file should have headers")
                .iter()
                .skip(match file_type {
                    Filetype::PolarACC => 2,
                    Filetype::PolarECG => 3,
                    Filetype::PolarHR | Filetype::PolarRR => 1,
                    Filetype::Unknown => return Err(ParserError::ContentError),
                })
                .for_each(|h| headers.push(h.to_string()));
        }
        let n_header_fields = headers.len();
        dbg!(&headers);
        dbg!(n_header_fields);

        let n_channel = match file_type {
            Filetype::PolarECG | Filetype::PolarHR | Filetype::PolarRR => 1,
            Filetype::PolarACC => 3,
            Filetype::Unknown => return Err(ParserError::ContentError),
        };

        (0..n_channel).for_each(|_header| {
            channels.push(Vec::with_capacity(n_records));
        });

        let mut record = StringRecord::new();
        while rdr.read_record(&mut record).is_ok() {
            // Phone TimeStamp
            // dbg!(&record);
            let mut x = NaiveDateTime::MIN;
            let fields: Vec<&str> = record.iter().collect();
            if fields.is_empty() {
                break;
            };
            if let Ok(datetime) =
                chrono::NaiveDateTime::parse_from_str(fields[0], "%Y-%m-%dT%H:%M:%S%.f")
            {
                x = datetime;
            } else {
                dbg!(&record);
                dbg!(&record.position());
            }
            match file_type {
                Filetype::PolarRR => {
                    let y = fields[1].parse::<f64>().unwrap() * 1E-3_f64;
                    channels[0].push((x, y));
                }
                Filetype::PolarHR => {
                    let y = fields[1].parse::<f64>().unwrap();
                    channels[0].push((x, y));
                }
                Filetype::PolarECG => {
                    let y = fields[3].parse::<f64>().unwrap() * 1E-3_f64;
                    // x = fields[2].parse::<f64>().unwrap() * 1E-3_f64;
                    channels[0].push((x, y));
                }
                Filetype::PolarACC => {
                    let y1 = fields[2].parse::<f64>().unwrap() / 1000.0;
                    let y2 = fields[3].parse::<f64>().unwrap() / 1000.0;
                    let y3 = fields[4].parse::<f64>().unwrap() / 1000.0;
                    channels[0].push((x, y1));
                    channels[1].push((x, y2));
                    channels[2].push((x, y3));
                }
                Filetype::Unknown => return Err(ParserError::ContentError),
            }
        }
        Ok(channels
            .drain(..)
            .zip(headers)
            .map(|(c, h)| {
                let plot_type = match file_type {
                    Filetype::PolarACC
                    | Filetype::PolarECG
                    | Filetype::PolarHR
                    | Filetype::PolarRR => PlotType::Line,
                    Filetype::Unknown => PlotType::Points,
                };
                let unit_start = h.find('[').unwrap_or(0);
                let unit_end = h.find(']').unwrap_or(h.len());
                let unit = h[unit_start..unit_end].to_string();
                TimeBasedChannel::new(h.to_string(), c, 1.0, plot_type, unit, None)
            })
            .collect())
    }
}

impl DrawableChannel for TimeBasedChannel {
    fn get_name(&mut self) -> String {
        self.name.to_string()
    }

    fn points_to_draw(&mut self, start_pos: f64, _end_pos: f64) -> PlotPoints {
        let _ = start_pos;
        // TODO use start_pos and end_pos
        self.data
            .iter()
            .map(|(x, y)| {
                [
                    x.timestamp_millis() as f64 / 1E3_f64,
                    *y * self.scaling_factor,
                ]
            })
            .collect()
    }

    fn draw(&mut self, plot_ui: &mut PlotUi, start_pos: f64, end_pos: f64) {
        match self.plot_type {
            // TODO get displayed bounds - is this a performance optimization?
            PlotType::Line => {
                let plot_points: PlotPoints = self.points_to_draw(start_pos, end_pos);
                let line = Line::new(plot_points)
                    .width(2.0)
                    .color(self.color)
                    .name(self.name.to_owned());
                plot_ui.line(line);
            }
            PlotType::Points => {
                let plot_points: PlotPoints = self
                    .data
                    .iter()
                    .filter(|(_idx, v)| *v == 0.0f64)
                    .map(|(x, y)| {
                        [
                            x.timestamp_millis() as f64 / 1E6_f64,
                            *y * self.scaling_factor,
                        ]
                    })
                    .collect();
                let points = Points::new(plot_points)
                    .color(self.color)
                    .radius(5.0)
                    .stems(0.0) // draw a line to 0
                    .name(self.name.to_owned());
                plot_ui.points(points);
            }
        }
    }

    fn show_settings(&mut self) {
        todo!();
    }

    fn get_unit(&mut self) -> String {
        self.unit.to_owned()
    }
}

#[derive(Clone, Debug)]
pub struct SampleBasedChannel {
    name: String,
    data: Vec<f64>,
    samples_per_second: f64,
    scaling_factor: f64,
    plot_type: PlotType,
    color: Color32,
    unit: String,
}

impl SampleBasedChannel {
    pub fn new(
        name: String,
        data: Vec<f64>,
        samples_per_second: f64,
        scaling_factor: f64,
        plot_type: PlotType,
        color: Option<Color32>,
        unit: String,
    ) -> SampleBasedChannel {
        SampleBasedChannel {
            name,
            data,
            samples_per_second,
            scaling_factor,
            plot_type,
            color: color.unwrap_or(Color32::TRANSPARENT),
            unit,
        }
    }

    pub fn get_slice(&mut self, start: Option<usize>, end: Option<usize>) -> &[f64] {
        let start = start.unwrap_or(0);
        let end = end.unwrap_or(self.data.len());
        &self.data[start..end]
    }

    pub fn parse_galaxy_data(
        data: String,
        n_records: usize,
    ) -> Result<Vec<SampleBasedChannel>, ParserError> {
        let mut line_it = data.lines();

        // get the name
        let name = line_it.next().unwrap();
        if let Some((_, name)) = name.split_once(',') {
            dbg!(&name);
            // get the date of birth
            let bday = line_it.next().unwrap();
            if let Some((_, bday)) = bday.split_once(',') {
                if let Ok(bday) = NaiveDate::parse_from_str(bday, "%Y-%m-%d") {
                    dbg!(&bday);
                    if let Some((_, avg_pulse)) = line_it.next().unwrap().split_once(',') {
                        let _avg_pulse = avg_pulse.parse::<f64>().unwrap_or(f64::NAN);
                        line_it.next(); // skip Unterteilung
                        line_it.next(); // skip Symptome
                        line_it.next(); // skip Software Version
                        line_it.next(); // skip Device
                        let samples_per_second = line_it
                            .next()
                            .unwrap()
                            .split_once(',')
                            .unwrap()
                            .1
                            .split_once(' ')
                            .unwrap()
                            .0
                            .parse::<f64>()
                            .unwrap_or(500_000.0)
                            / 1000.0;

                        // skip empty lines
                        line_it.next();
                        line_it.next();
                        // skip channel description
                        line_it.next();
                        line_it.next();

                        let mut data = Vec::with_capacity(n_records);
                        data = line_it
                            .map(|line| line.replace(',', ".").parse::<f64>().unwrap_or(f64::NAN))
                            .collect();

                        let scaling_factor = 1.0;
                        let color = None;
                        let unit = "mV".to_owned();
                        let plot_type = PlotType::Line;

                        let name = format!("{} {}", name, bday);

                        return Ok(vec![SampleBasedChannel::new(
                            name,
                            data,
                            samples_per_second,
                            scaling_factor,
                            plot_type,
                            color,
                            unit,
                        )]);
                    }
                }
            }
        };
        Err(ParserError::ContentError)
    }
}

impl DrawableChannel for SampleBasedChannel {
    fn points_to_draw(&mut self, start_pos: f64, end_pos: f64) -> PlotPoints {
        // make sure our slice is within bounds
        let start_idx: usize = ((start_pos.max(0.0f64) * self.samples_per_second).floor() as usize)
            .min(self.data.len());
        let end_idx: usize =
            ((end_pos.max(0.0f64) * self.samples_per_second).ceil() as usize).min(self.data.len());
        assert!(start_idx <= end_idx);
        self.data[start_idx..end_idx]
            .iter()
            .enumerate()
            .map(|(idx, y)| {
                [
                    idx as f64 / self.samples_per_second,
                    y * self.scaling_factor,
                ]
            })
            .collect()
    }

    fn draw(&mut self, plot_ui: &mut PlotUi, start_pos: f64, end_pos: f64) {
        match self.plot_type {
            // TODO get displayed bounds - is this a performance optimization?
            PlotType::Line => {
                let plot_points: PlotPoints = self.points_to_draw(start_pos, end_pos);
                let line = Line::new(plot_points)
                    .width(2.0)
                    .color(self.color)
                    .name(self.name.to_owned());
                plot_ui.line(line);
            }
            PlotType::Points => {
                let plot_points: PlotPoints = self
                    .data
                    .iter()
                    .enumerate()
                    .filter(|(_idx, v)| **v == 0.0f64)
                    .map(|(idx, _y)| {
                        [
                            idx as f64 / self.samples_per_second,
                            1.0 * self.scaling_factor,
                        ]
                    })
                    .collect();
                let points = Points::new(plot_points)
                    .color(self.color)
                    .radius(5.0)
                    .stems(0.0) // draw a line to 0
                    .name(self.name.to_owned());
                plot_ui.points(points);
            }
        }
    }

    fn get_unit(&mut self) -> String {
        self.unit.to_owned()
    }

    fn get_name(&mut self) -> String {
        self.name.to_owned()
    }

    fn show_settings(&mut self) {
        todo!();
    }
}
