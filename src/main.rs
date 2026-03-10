use std::{
    os::fd::AsRawFd, ptr::slice_from_raw_parts_mut, str::from_utf8_unchecked,
};

use mmap::{MapOption, MemoryMap};
use rapidhash::{HashMapExt, RapidHashMap};

fn main() {
    let file = std::fs::File::open("./data/measurements.txt").unwrap();
    let file_len = file.metadata().unwrap().len() as usize;
    let mmap = MemoryMap::new(
        file_len,
        &[MapOption::MapFd(file.as_raw_fd()), MapOption::MapReadable],
    )
    .unwrap();
    let slice = slice_from_raw_parts_mut(mmap.data(), file_len);
    let contents = unsafe { from_utf8_unchecked(slice.as_ref().unwrap()) }.trim();

    // (min, max, len, total)
    let mut map: RapidHashMap<&str, (f64, f64, usize, f64)> = RapidHashMap::with_capacity(1000);
    for line in contents.lines() {
        let (station, temperature) = split_stat(line);
        let temperature = parse_temperature(temperature);

        let entry = map
            .entry(station)
            .or_insert_with(|| (f64::MAX, f64::MIN, 0, 0.));
        entry.0 = entry.0.min(temperature);
        entry.1 = entry.1.max(temperature);
        entry.2 += 1;
        entry.3 += temperature;
    }

    print!("{{");
    let mut sorted: Vec<&str> = Vec::with_capacity(1_000_000_000);
    sorted.extend(map.keys());
    sorted.sort_unstable();

    let mut key_iter = sorted.iter().peekable();
    while let Some(key) = key_iter.next() {
        let (min, max, len, sum) = map.get(key).unwrap();
        let mean = sum / *len as f64;
        print!("{key}={min:.1}/{mean:.1}/{max:.1}");
        if key_iter.peek().is_some() {
            print!(", ");
        }
    }

    println!("}}");
}

#[inline(always)]
fn split_stat(stat: &str) -> (&str, &str) {
    let index = stat.chars().rev().position(|c| c == ';').unwrap();
    stat.split_at(stat.len() - index)
}

#[inline(always)]
fn parse_temperature(temperature: &str) -> f64 {
    let mut chars = temperature.chars().rev().peekable();
    let decimal = chars.next().unwrap() as i32 - '0' as i32;
    let negative = unsafe { *temperature.as_ptr() == b'-' };

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
