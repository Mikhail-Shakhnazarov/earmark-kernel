#![no_main]
use earmark_core::docs::unwrap_hard_wrapped_prose;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = unwrap_hard_wrapped_prose(s);
    }
});
