#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(out) = fjson::format_json_compact(s) {
            match serde_json::from_str::<serde_json::Value>(&out) {
                Ok(_) => {}
                Err(err) => {
                    let s = err.to_string();
                    if s.contains("number out of range")
                        || s.contains("lone leading surrogate in hex escape")
                        || s.contains("unexpected end of hex escape")
                    {
                        return;
                    }
                    panic!("{}", err);
                }
            }
        }
    }
});
