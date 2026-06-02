#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _: Result<earmark_core::ObjectRecord, _> = serde_json::from_str(s);
        let _: Result<earmark_core::RunRecord, _> = serde_json::from_str(s);
        let _: Result<earmark_core::PacketRecord, _> = serde_json::from_str(s);
    }
});
