use crate::data_structures::{Filetype, SampleBasedChannel, TimeBasedChannel};

pub fn parse_content(text: Vec<u8>) -> (Vec<SampleBasedChannel>, Vec<TimeBasedChannel>) {
    let mut sample_based_data_channels: Vec<SampleBasedChannel> = vec![];
    let mut time_based_data_channels: Vec<TimeBasedChannel> = vec![];

    if !text.is_empty() {
        let utf_string = String::from_utf8_lossy(&text).to_string();
        let n_records = utf_string.chars().filter(|c| *c == '\n').count() + 1;

        /*
        check the first line of the file content to see if it has a know header

        The Polar files are time-based, so we create a TimeBasedChannel for them
        */

        match utf_string.split_once('\n').unwrap_or_default().0.trim() {
            "Phone timestamp;sensor timestamp [ns];timestamp [ms];ecg [uV]" => {
                println!("Polar X10 ECG");
                if let Ok(channels) =
                    TimeBasedChannel::parse_polar_data(utf_string, Filetype::PolarECG, n_records)
                {
                    time_based_data_channels.extend(channels);
                }
            }
            "Phone timestamp;sensor timestamp [ns];X [mg];Y [mg];Z [mg]" => {
                println!("Polar X10 ACC");
                if let Ok(channels) =
                    TimeBasedChannel::parse_polar_data(utf_string, Filetype::PolarACC, n_records)
                {
                    time_based_data_channels.extend(channels);
                }
            }
            "Phone timestamp;HR [bpm]" => {
                println!("Polar X10 HR");
                if let Ok(channels) =
                    TimeBasedChannel::parse_polar_data(utf_string, Filetype::PolarHR, n_records)
                {
                    time_based_data_channels.extend(channels);
                }
            }
            "Phone timestamp;RR-interval [ms]" => {
                println!("Polar X10 RR");
                if let Ok(channels) =
                    TimeBasedChannel::parse_polar_data(utf_string, Filetype::PolarRR, n_records)
                {
                    time_based_data_channels.extend(channels);
                }
            }
            _ => {
                println!("unknown data type");
                // Filetype::Unknown
            }
        };
    };

    // let _ = sender.send(utf_string);
    (sample_based_data_channels, time_based_data_channels)
}
