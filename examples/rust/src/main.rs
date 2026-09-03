use universal_payment_qr_core::parse_payment_qr;

fn main() {
    let payload = std::env::args().nth(1).expect("pass a decoded QR payload");
    println!("{}", serde_json::to_string_pretty(&parse_payment_qr(&payload)).unwrap());
}

