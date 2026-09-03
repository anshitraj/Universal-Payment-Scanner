use serde::Deserialize;
use universal_payment_qr_core::parse_payment_qr;

#[derive(Deserialize)]
struct Vector {
    name: String,
    payload: String,
    expected: Expected,
}

#[derive(Deserialize)]
struct Expected {
    recognized: bool,
    scheme: String,
    valid: bool,
    amount: Option<String>,
    error: Option<String>,
}

#[test]
fn shared_vectors_match_contract() {
    let vectors: Vec<Vector> =
        serde_json::from_str(include_str!("../../../test-vectors/v1.json")).unwrap();
    for vector in vectors {
        let result = parse_payment_qr(&vector.payload);
        assert_eq!(
            result.recognized, vector.expected.recognized,
            "{}",
            vector.name
        );
        assert_eq!(result.scheme, vector.expected.scheme, "{}", vector.name);
        assert_eq!(
            result.validation.valid, vector.expected.valid,
            "{}",
            vector.name
        );
        if let Some(amount) = vector.expected.amount {
            assert_eq!(
                result.amount.as_deref(),
                Some(amount.as_str()),
                "{}",
                vector.name
            );
        }
        if let Some(error) = vector.expected.error {
            assert_eq!(
                format!("{:?}", result.validation.errors[0].code).to_uppercase(),
                error.replace('_', ""),
                "{}",
                vector.name
            );
        }
    }
}
