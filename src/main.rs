use std::{
    arch::x86_64::{
        _MM_HINT_T0, _MM_HINT_T1, _mm_prefetch, _mm256_cmpeq_epi8, _mm256_loadu_epi8, _mm256_movemask_epi8, _mm256_set1_epi8
    },
    os::fd::AsRawFd,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
    str::from_utf8_unchecked,
};

use mmap::{MapOption, MemoryMap};

const N: usize = 1 << 17;

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

    let mut table = vec![Entry::default(); N];
    let seed = 123456;

    const CHUNKSIZE: usize = 32;
    let needle = unsafe { _mm256_set1_epi8(b'\n' as i8) };
    let ptr = bytes.as_ptr();
    let bytes_len = bytes.len();
    let mut i = 0;
    let mut chunks = 0;
    let mut found = false;

    // TODO: semicolon on same pass?
    'main_loop: while i + CHUNKSIZE <= bytes_len {
        let line = unsafe {
            #[allow(unused)]
            let mut n = 0;
            while i + CHUNKSIZE <= bytes_len {
                let haystack = _mm256_loadu_epi8(ptr.add(i) as *const i8);
                let res = _mm256_cmpeq_epi8(needle, haystack);
                let mask = _mm256_movemask_epi8(res);
                if mask != 0 {
                    n = mask.trailing_zeros();
                    // newline is at i + n
                    found = true;
                    break;
                }
                chunks += 1;
                i += CHUNKSIZE;
            }

            if !found {
                break 'main_loop;
            }

            let chunk_offset = CHUNKSIZE * chunks;
            let line = slice_from_raw_parts(ptr.add(i - chunk_offset), chunk_offset + n as usize)
                .as_ref()
                .unwrap();
            i += 1 + n as usize;
            chunks = 0;
            line
        };

        let (station, temperature) = split_stat(line);
        let hash = gxhash::gxhash64(station, seed);
        let slot = (hash as usize) & (N - 1);
        unsafe {
            _mm_prefetch::<{ _MM_HINT_T0 }>(table.as_ptr().add(slot) as *const i8);
        }
        let temperature = parse_temperature(temperature);

        set_entry(&mut table, slot, hash, station, temperature);
    }

    if i < bytes_len {
        let lines = unsafe {
            let remainder = slice_from_raw_parts(ptr.add(i), bytes_len - i)
                .as_ref()
                .unwrap();
            remainder.split(|c| *c == b'\n')
        };

        for line in lines {
            let (station, temperature) = split_stat(line);
            let temperature = parse_temperature(temperature);

            let hash = gxhash::gxhash64(station, seed);
            let slot = (hash as usize) & (N - 1);
            set_entry(&mut table, slot, hash, station, temperature);
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

#[inline(always)]
fn set_entry(
    table: &mut [Entry],
    mut slot: usize,
    hash: u64,
    station: &'static [u8],
    temperature: i32,
) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // split_stat tests
    #[test]
    fn test_split_stat_single_digit_temp() {
        let (station, temp) = split_stat(b"Darwin;1.2");
        assert_eq!(station, b"Darwin");
        assert_eq!(temp, b"1.2");
    }

    #[test]
    fn test_split_stat_double_digit_temp() {
        let (station, temp) = split_stat(b"Darwin;32.1");
        assert_eq!(station, b"Darwin");
        assert_eq!(temp, b"32.1");
    }

    #[test]
    fn test_split_stat_negative_single_digit_temp() {
        let (station, temp) = split_stat(b"Darwin;-1.2");
        assert_eq!(station, b"Darwin");
        assert_eq!(temp, b"-1.2");
    }

    #[test]
    fn test_split_stat_negative_double_digit_temp() {
        let (station, temp) = split_stat(b"Darwin;-32.1");
        assert_eq!(station, b"Darwin");
        assert_eq!(temp, b"-32.1");
    }

    #[test]
    fn test_split_stat_multibyte_station() {
        // Chișinău contains ș (0xC8 0x99) and ă (0xC4 0x83)
        let input = "Chișinău;1.7".as_bytes();
        let (station, temp) = split_stat(input);
        assert_eq!(station, "Chișinău".as_bytes());
        assert_eq!(temp, b"1.7");
    }

    #[test]
    fn test_split_stat_multibyte_station_negative_temp() {
        let input = "Chișinău;-32.1".as_bytes();
        let (station, temp) = split_stat(input);
        assert_eq!(station, "Chișinău".as_bytes());
        assert_eq!(temp, b"-32.1");
    }

    // parse_temperature tests — values stored as fixed-point × 10
    #[test]
    fn test_parse_temperature_single_digit() {
        assert_eq!(parse_temperature(b"1.2"), 12);
    }

    #[test]
    fn test_parse_temperature_double_digit() {
        assert_eq!(parse_temperature(b"32.1"), 321);
    }

    #[test]
    fn test_parse_temperature_negative_single_digit() {
        assert_eq!(parse_temperature(b"-1.2"), -12);
    }

    #[test]
    fn test_parse_temperature_negative_double_digit() {
        assert_eq!(parse_temperature(b"-32.1"), -321);
    }

    #[test]
    fn test_parse_temperature_zero() {
        assert_eq!(parse_temperature(b"0.0"), 0);
    }

    #[test]
    fn test_parse_temperature_max() {
        assert_eq!(parse_temperature(b"99.9"), 999);
    }

    #[test]
    fn test_parse_temperature_min() {
        assert_eq!(parse_temperature(b"-99.9"), -999);
    }
}
