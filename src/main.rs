use std::{os::fd::AsRawFd, ptr::slice_from_raw_parts_mut, str::from_utf8_unchecked};

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
    let bytes = unsafe { slice.as_ref().unwrap() };
    let contents = unsafe { from_utf8_unchecked(bytes) };

    // (min, max, len, total)
    let mut map: RapidHashMap<&str, (f64, f64, usize, f64)> = RapidHashMap::with_capacity(1000);
    // TODO: raw byte reading
    for line in contents.lines() {
        let (station, temperature) = split_stat(line);
        let temperature = parse_temperature(temperature.as_bytes());

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
    let index = stat.bytes().rposition(|c| c == b';').unwrap();
    let (left, right) = stat.split_at(index);
    (left, &right[1..])
}

#[inline(always)]
fn parse_temperature(temperature: &[u8]) -> f64 {
    let len = temperature.len();
    let negative = unsafe { *temperature.get_unchecked(0) == b'-' };
    let decimal = unsafe { *temperature.get_unchecked(len - 1) } - b'0';
    let ndigits = len - 2 - negative as usize;
    let start_index = negative as usize;

    // either 1 or 2
    let whole = if ndigits == 1 {
        (unsafe { *temperature.get_unchecked(start_index) } - b'0')
    } else {
        (unsafe { *temperature.get_unchecked(start_index) } - b'0') * 10
            + (unsafe { *temperature.get_unchecked(start_index + 1) } - b'0')
    };

    ((whole as f64) + decimal as f64 / 10.) * (1 - 2 * negative as i32) as f64
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_parse_temperature() {
        // negative two-digit whole
        assert_eq!(parse_temperature(b"-99.9"), -99.9);
        assert_eq!(parse_temperature(b"-99.0"), -99.0);
        assert_eq!(parse_temperature(b"-50.5"), -50.5);
        assert_eq!(parse_temperature(b"-10.1"), -10.1);
        assert_eq!(parse_temperature(b"-12.3"), -12.3);
        // negative one-digit whole
        assert_eq!(parse_temperature(b"-9.9"), -9.9);
        assert_eq!(parse_temperature(b"-9.0"), -9.0);
        assert_eq!(parse_temperature(b"-1.5"), -1.5);
        assert_eq!(parse_temperature(b"-0.1"), -0.1);
        // zero
        assert_eq!(parse_temperature(b"0.0"), 0.0);
        // positive one-digit whole
        assert_eq!(parse_temperature(b"0.1"), 0.1);
        assert_eq!(parse_temperature(b"1.5"), 1.5);
        assert_eq!(parse_temperature(b"9.0"), 9.0);
        assert_eq!(parse_temperature(b"9.9"), 9.9);
        // positive two-digit whole
        assert_eq!(parse_temperature(b"10.0"), 10.0);
        assert_eq!(parse_temperature(b"12.3"), 12.3);
        assert_eq!(parse_temperature(b"50.5"), 50.5);
        assert_eq!(parse_temperature(b"99.0"), 99.0);
        assert_eq!(parse_temperature(b"99.9"), 99.9);
    }
}
