// C ABI matching crates/ffi/src/lib.rs exactly - this header IS the contract between that crate
// and this package; if one changes, the other must change with it. See that crate's module docs
// for the full memory-ownership contract (every non-null return here must be passed to
// upqr_free_string exactly once).
#ifndef UNIVERSAL_PAYMENT_QR_FFI_H
#define UNIVERSAL_PAYMENT_QR_FFI_H

#ifdef __cplusplus
extern "C" {
#endif

// Parses a payload with the default policy (policy_json NULL) or a JSON-encoded
// CapabilityPolicy override. Returns a JSON-encoded PaymentIntent.
char *upqr_parse(const char *payload, const char *policy_json);

// Detects a payload without full parsing. Returns a JSON-encoded Detection.
char *upqr_detect(const char *payload, const char *policy_json);

// Every registered scheme's metadata, independent of which are enabled by policy_json.
// Returns a JSON-encoded array.
char *upqr_schemes(const char *policy_json);

// The crate version (CARGO_PKG_VERSION).
char *upqr_version(void);

// Frees a string previously returned by any upqr_* function above. Safe to call with NULL.
void upqr_free_string(char *ptr);

#ifdef __cplusplus
}
#endif

#endif
