use std::{collections::HashMap, fs::read_to_string};

fn main() {
    let contents = read_to_string("./data/measurements.txt").unwrap();
    // (min, max, len, total)
    let mut map: HashMap<String, (f64, f64, usize, f64)> = HashMap::new();
    for line in contents.lines() {
        let (station, temperature) = line.split_once(";").unwrap();
        let temperature = temperature.parse::<f64>().unwrap();

        let entry = map.entry(station.to_string())
            .or_insert((f64::MAX, f64::MIN, 0, 0.));
        entry.0 = entry.0.min(temperature);
        entry.1 = entry.1.max(temperature);
        entry.2 += 1;
        entry.3 += temperature;
    }

    print!("{{");
    let mut stats = map.into_iter().peekable();
    while let Some((station, (min, max, len, sum))) = stats.next() {

        let mean = sum / len as f64;
        print!("{station}={min:.1}/{mean:.1}/{max:.1}");
        if stats.peek().is_some() {
            print!(", ");
        }
    }
    println!("}}");
}
