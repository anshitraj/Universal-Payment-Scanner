#![no_main]
use libfuzzer_sys::fuzz_target;
use universal_payment_qr_core::parse_payment_qr;

fuzz_target!(|data: &[u8]| {
    let payload = String::from_utf8_lossy(data);
    let _ = parse_payment_qr(&payload);
});

