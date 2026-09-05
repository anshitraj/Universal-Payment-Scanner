## 0.1.0

- Initial release. Dart FFI bindings over `crates/ffi`'s C ABI: `parsePaymentQr`, `detectPaymentQr`,
  `getCapabilities`, `getSupportedSchemes`, and a `Scanner` class matching the shape of every other
  Universal Payment QR binding (npm, Python, Kotlin/Android).
- Verified with 8/8 `flutter test` against a host-built native library in this repository's own
  development session. Android/iOS packaging of the cross-compiled native library is not yet built
  - see the package README for exactly what that means for `flutter build apk`/`ios` today.
