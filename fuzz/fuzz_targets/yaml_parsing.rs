#![no_main]
use earmark_core::ObjectRecord;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _: Result<ObjectRecord, _> = serde_yaml::from_str(s);
    }
});
