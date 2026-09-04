# UniversalPaymentQR (Swift)

Swift bindings over the same Rust core every SDK in this project uses (`crates/core`), via the C
ABI in `crates/ffi`. No parsing logic is reimplemented here.

```swift
import UniversalPaymentQR

let result = try UniversalPaymentQR.parse("upi://pay?pa=merchant%40bank&am=499.00&cu=INR")
result["scheme"] as? String // "upi"
```

```swift
let scanner = createScanner(schemes: ["upi": true, "bitcoin": false])
try scanner.scan(payload)
```

## What wasn't verified here

**Nothing in this package has been compiled.** This repository was developed on Windows; Xcode
and the Swift toolchain only run on macOS, so neither `Package.swift` nor the Swift source here
was checked with a real compiler. The C header (`Sources/CUniversalPaymentQR/include/*.h`)
matches `crates/ffi/src/lib.rs`'s exported symbols field-for-field, and the Swift wrapper follows
standard, common patterns (`withCString`, `JSONSerialization`, `defer`-based cleanup) - but "written
carefully" is not the same claim as "verified," and it should be compiled and run through
`Tests/UniversalPaymentQRTests` on a Mac before anyone relies on it.

## Building (on a Mac)

`Package.swift` links against a static library this repository doesn't build for you yet - `cargo
build` alone only produces a library for your host OS, not iOS. Building a real XCFramework needs
Xcode's toolchain:

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
cd crates/ffi

cargo build --release --target aarch64-apple-ios        # device
cargo build --release --target aarch64-apple-ios-sim    # simulator (Apple Silicon)
cargo build --release --target x86_64-apple-ios         # simulator (Intel)

# Combine the two simulator slices into one fat binary, then package everything as an XCFramework:
lipo -create \
  ../../target/aarch64-apple-ios-sim/release/libuniversal_payment_qr_ffi.a \
  ../../target/x86_64-apple-ios/release/libuniversal_payment_qr_ffi.a \
  -output libuniversal_payment_qr_ffi-sim.a

xcodebuild -create-xcframework \
  -library ../../target/aarch64-apple-ios/release/libuniversal_payment_qr_ffi.a \
  -headers Sources/CUniversalPaymentQR/include \
  -library libuniversal_payment_qr_ffi-sim.a \
  -headers Sources/CUniversalPaymentQR/include \
  -output UniversalPaymentQRFFI.xcframework
```

Then update `Package.swift` to reference `UniversalPaymentQRFFI.xcframework` as a `.binaryTarget`
instead of the placeholder `linkerSettings` in the `CUniversalPaymentQR` target, and
`CUniversalPaymentQR` becomes unnecessary (the xcframework carries its own headers). Run
`swift test` to execute `Tests/UniversalPaymentQRTests` - the same assertions the Python, Kotlin,
and Dart bindings already pass in this repository.
