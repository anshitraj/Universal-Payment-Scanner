use std::hint::black_box;
use universal_payment_qr_core::parse_payment_qr;

fn main() {
    let cases = [
        "upi://pay?pa=merchant%40bank&pn=Acme&am=499.00&cu=INR",
        "bitcoin:1BoatSLRHtKNngkdXEeobR76b53LETtpyT?amount=0.001",
        "ethereum:0xfb6916095ca1df60bb79Ce92ce3ea74c37c5d359?value=1e18",
        "solana:9xQeWvG816bUx9EPfEZi4q3G44E2sA6v7a6D9k5GmRse?amount=1.25",
    ];
    let start = std::time::Instant::now();
    let iterations = 100_000usize;
    for i in 0..iterations {
        black_box(parse_payment_qr(black_box(cases[i % cases.len()])));
    }
    let elapsed = start.elapsed();
    println!(
        "{} parses in {:?} ({:?}/parse)",
        iterations,
        elapsed,
        elapsed / iterations as u32
    );
}
