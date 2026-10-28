//! The CSV importer with detected column mappings: arbitrary input never
//! panics, and whatever it accepts makes valid items (`common::check`). The
//! first byte picks an explicit mapping for the rest of the input instead,
//! so hidden, OTP, tag and favorite columns are reached too. Seeds: the
//! Chrome, Firefox, Safari, Bitwarden and German exports of
//! `crates/import/tests/formats.rs`.
#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;
use mosaic_import::csv::{CsvColumn, CsvMapping, parse};

fuzz_target!(|data: &[u8]| {
    common::check(parse(data, None));
    if let Some((&mode, rest)) = data.split_first() {
        let all = [
            CsvColumn::Title,
            CsvColumn::Username,
            CsvColumn::Password,
            CsvColumn::Url,
            CsvColumn::Notes,
            CsvColumn::Otp,
            CsvColumn::Folder,
            CsvColumn::Tags,
            CsvColumn::Favorite,
            CsvColumn::Text("t".into()),
            CsvColumn::Hidden("h".into()),
            CsvColumn::Ignore,
        ];
        let columns = (0..8)
            .map(|i| all[usize::from(mode.rotate_left(i)) % all.len()].clone())
            .collect();
        common::check(parse(rest, Some(&CsvMapping { columns })));
    }
});
