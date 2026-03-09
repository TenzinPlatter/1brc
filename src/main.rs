use std::{
    collections::HashMap,
    hash::{BuildHasherDefault, Hasher},
    os::fd::AsRawFd,
    ptr::slice_from_raw_parts_mut,
    str::from_utf8_unchecked,
};

use mmap::{MapOption, MemoryMap};

#[derive(Default)]
struct MyHasher(u64);

impl Hasher for MyHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        // hash := FNV_offset_basis

        // for each byte_of_data to be hashed do
        //     hash := hash × FNV_prime
        //     hash := hash XOR byte_of_data
        let mut hash: u64 = 14695981039346656037;
        for byte in bytes.iter() {
            hash *= 1099511628211;
            hash ^= *byte as u64;
        }

        self.0 = hash
    }
}

type MyMap<K, V> = HashMap<K, V, BuildHasherDefault<MyHasher>>;

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
    // let mut map: MyMap<&str, (f64, f64, usize, f64)> = MyMap::default();
    let mut map: HashMap<&str, (f64, f64, usize, f64)> = HashMap::default();
    for line in contents.lines() {
        let (station, temperature) = split_stat(line);
        let temperature = parse_temperature(temperature);

        let entry = map.entry(station).or_insert((f64::MAX, f64::MIN, 0, 0.));
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
