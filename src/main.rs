use std::{collections::HashMap, fs::read_to_string};

fn main() {
    let contents = read_to_string("./data/measurements.txt").unwrap();
    let mut map: HashMap<String, Vec<f32>> = HashMap::new();
    for line in contents.lines() {
        let (station, temperature) = line.split_once(";").unwrap();
        let temperature = temperature.parse::<f32>().unwrap();

        if map.contains_key(station) {
            let last = map.get_mut(station).unwrap();
            last.push(temperature);
        } else {
            map.insert(station.to_string(), vec![temperature]);
        }
    }

    print!("{{");
    let n_stations = map.len();
    for (curr, (station, measurements)) in map.into_iter().enumerate() {
        let min = measurements
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let max = measurements
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        let len = measurements.len();
        let mean = measurements.iter().sum::<f32>() / len as f32;

        print!("{station}={min:.1}/{mean:.1}/{max:.1}");
        if curr < n_stations {
            print!(", ");
        }
    }
    println!("}}");
}
