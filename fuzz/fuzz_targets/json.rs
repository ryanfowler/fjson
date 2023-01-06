#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(out) = fjson::format_json_compact(s) {
            serde_json::from_str::<serde_json::Value>(&out)
                .expect("unable to parse output with serde_json");
        }
    }
});
