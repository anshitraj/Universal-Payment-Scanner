use std::io::{self, Read};

use universal_payment_qr_core::{CapabilityPolicy, Scanner};

fn main() {
    if let Err(error) = run() {
        eprintln!("upqr-core: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "parse".into());
    let policy = args
        .next()
        .map(|json| serde_json::from_str::<CapabilityPolicy>(&json))
        .transpose()?
        .unwrap_or_default();
    let scanner = Scanner::new(policy);
    let output = match command.as_str() {
        "schemes" | "capabilities" => serde_json::to_value(scanner.schemes())?,
        "parse" | "validate" | "detect" => {
            let mut payload = String::new();
            io::stdin().read_to_string(&mut payload)?;
            let payload = payload.trim_end_matches(['\r', '\n']);
            match command.as_str() {
                "detect" => serde_json::to_value(scanner.detect(payload))?,
                "validate" => serde_json::to_value(scanner.scan(payload).validation)?,
                _ => serde_json::to_value(scanner.scan(payload))?,
            }
        }
        _ => return Err(format!("unknown command: {command}").into()),
    };
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
