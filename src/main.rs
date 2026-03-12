use std::{
    arch::x86_64::{_mm_loadu_epi8, _mm256_loadu_epi8},
    os::fd::AsRawFd,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
    str::from_utf8_unchecked,
};

use mmap::{MapOption, MemoryMap};

const N: usize = 1 << 17;
const NLINES: usize = 1_000_000_000;

#[derive(Default, Clone)]
struct Entry {
    hash: u64,
    key: &'static [u8],
    min: i32,
    max: i32,
    sum: i32,
    count: i32,
}

fn main() {
    let file = std::fs::File::open("./data/measurements.txt").unwrap();
    let file_len = file.metadata().unwrap().len() as usize;
    let mmap = MemoryMap::new(
        file_len,
        &[MapOption::MapFd(file.as_raw_fd()), MapOption::MapReadable],
    )
    .unwrap();
    let slice = slice_from_raw_parts_mut(mmap.data(), file_len);
    let bytes = unsafe { slice.as_ref().unwrap().trim_ascii_end() };

    // (min, max, len, total)
    let mut table = vec![Entry::default(); N];

    const CHUNKSIZE: usize = 16;
    let ptr = bytes.as_ptr();
    let mut i = 0;

    // TODO: semicolon on same pass?
    // let newline_needle = unsafe { _mm256_loadu_epi8(b'\n' as i8) };
    // while i + CHUNKSIZE <= NLINES {
    for line in bytes.split(|c| *c == b'\n') {
        let (station, temperature) = split_stat(line);
        let temperature = parse_temperature(temperature);

        let seed = 123456;
        let hash = gxhash::gxhash64(station, seed);
        let mut slot = (hash as usize) & (N - 1);
        loop {
            let e = unsafe { table.get_unchecked_mut(slot) };
            if e.count == 0 {
                e.hash = hash;
                e.key = station;

                e.count += 1;
                e.min = temperature;
                e.max = temperature;
                e.sum = temperature;
                break;
            }
            if e.hash == hash && e.key == station {
                e.count += 1;
                e.min = e.min.min(temperature);
                e.max = e.max.max(temperature);
                e.sum += temperature;
                break;
            }
            slot = (slot + 1) & (N - 1);
        }
    }

    print!("{{");
    table.sort_unstable_by(|a, b| a.key.partial_cmp(b.key).unwrap());
    let mut iter = table.iter().filter(|i| i.count > 0).peekable();
    while let Some(e) = iter.next() {
        let min = e.min as f64 / 10.;
        let max = e.max as f64 / 10.;
        let mean = (e.sum as f64 / 10.) / e.count as f64;
        print!(
            "{station}={min:.1}/{mean:.1}/{max:.1}",
            station = unsafe { from_utf8_unchecked(e.key) }
        );

        if iter.peek().is_some() {
            print!(", ");
        }
    }

    println!("}}");
}

#[inline(always)]
fn split_stat(stat: &[u8]) -> (&[u8], &[u8]) {
    // max len: -99.9 - 5 bytes - ';' is at most 6 bytes from end
    // min len: 0.0 - 3 bytes - ';' is at least 4 bytes
    // i.e. ';' is at one of len - {4,5,6}
    let len = stat.len();
    let at_4 = unsafe { *stat.get_unchecked(len - 4) } == b';';
    let at_5 = unsafe { *stat.get_unchecked(len - 5) } == b';';
    let at_6 = unsafe { *stat.get_unchecked(len - 6) } == b';';

    let index = len - (at_4 as usize * 4) - (at_5 as usize * 5) - (at_6 as usize * 6);
    let ptr = stat.as_ptr();
    unsafe {
        let left = slice_from_raw_parts(ptr, index);
        let right = slice_from_raw_parts(ptr.add(index + 1), len - index - 1);
        (left.as_ref().unwrap(), right.as_ref().unwrap())
    }
}

#[inline(always)]
fn parse_temperature(temperature: &[u8]) -> i32 {
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

    ((whole as i32 * 10) + decimal as i32) * (1 - 2 * negative as i32)
}
