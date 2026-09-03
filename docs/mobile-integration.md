# Mobile integration

The Rust core has no OS or camera dependency. Mobile applications should decode the QR with their
platform camera framework, then pass the raw string to a native Rust binding or an embedded web
view using the WASM SDK.

- iOS: use `AVCaptureMetadataOutput` for decoding. Generate Swift bindings around the Rust crate
  with a project-approved FFI tool; do not open the decoded URL automatically.
- Android: use CameraX/ML Kit or ZXing for decoding. Feed the decoded string into the Rust binding;
  request camera permission only while scanning.
- React Native/Capacitor: use an existing camera decoder and call `scanner.scan(rawString)` in the
  JavaScript runtime. Keep provider handoff behind an explicit user action.

Native binding packages are not shipped in v0.1, so mobile consumers currently need to own the FFI
packaging step. Cross-language fixtures in `test-vectors/` are the compatibility contract.

