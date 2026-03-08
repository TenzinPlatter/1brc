use std::{collections::HashMap, fs::read_to_string};

fn main() {
    let contents = read_to_string("./data/measurements.txt").unwrap();
    // (min, max, len, total)
    let mut map: HashMap<&str, (f64, f64, usize, f64)> = HashMap::new();
    for line in contents.lines() {
        let (station, temperature) = line.rsplit_once(";").unwrap();
        let temperature = parse_temperature(temperature);

        let entry = map.entry(station).or_insert((f64::MAX, f64::MIN, 0, 0.));
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

fn parse_temperature(temperature: &str) -> f64 {
    let mut chars = temperature.chars().rev().peekable();
    let decimal = chars.next().unwrap() as i32 - '0' as i32;
    let negative = unsafe {
        *temperature.as_ptr() == b'-'
    };

    // skip over the '.' character
    let _ = chars.next();
    let mut whole = 0;
    while let Some(c) = chars.next() {
        if chars.peek().is_none() && negative {
            break;
        }

        whole += c as i32 - '0' as i32;
    }

    if negative {
        -((whole as f64) + decimal as f64 / 10.)
    } else {
        (whole as f64) + decimal as f64 / 10.
    }
}
