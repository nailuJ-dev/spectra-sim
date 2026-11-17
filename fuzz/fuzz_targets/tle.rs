#![no_main]

use libfuzzer_sys::fuzz_target;

// Input layout: up to three newline-separated lines. With three or more lines
// the first is the object name and the next two are TLE lines 1 and 2; with
// exactly two lines they are TLE lines 1 and 2 and there is no name.
fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut lines = text.splitn(3, '\n');
    let (Some(first), Some(second)) = (lines.next(), lines.next()) else {
        return;
    };
    match lines.next() {
        Some(third) => {
            let third = third.split('\n').next().unwrap_or("");
            let _ = spectra_sim::Tle::parse(Some(first.to_string()), second, third);
        }
        None => {
            let _ = spectra_sim::Tle::parse(None, first, second);
        }
    }
});
