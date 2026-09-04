# universal_payment_qr (Flutter)

`dart:ffi` bindings over the same Rust core every SDK in this project uses (`crates/core`), via
the generic C ABI in `crates/ffi` - no JNI needed, Dart FFI calls C directly. No parsing logic is
reimplemented here.

```dart
import 'package:universal_payment_qr/universal_payment_qr.dart';

final result = UniversalPaymentQr.parse('upi://pay?pa=merchant%40bank&am=499.00&cu=INR');
print(result['scheme']); // upi
```

```dart
final scanner = createScanner(schemes: {'upi': true, 'bitcoin': false});
scanner.scan(payload);
```

## Verified in this session

`flutter test` (8/8 tests) against a real host build of `crates/ffi`, using
`libraryPathOverride` to point at it directly - see `test/universal_payment_qr_test.dart`. That
proves the Dart↔Rust FFI contract (types, JSON shape, memory ownership) is correct end to end.

## Not verified in this session

- **Android**: `android/build.gradle` bundles the `.so` files from
  `target/android-jniLibs/` (build them with the `cargo ndk` command in the repository root
  README's Android section), and `NativeBindings` opens `libuniversal_payment_qr_ffi.so` by name
  on `Platform.isAndroid` - that resolution logic was not exercised against a real device/emulator
  or a full `flutter build apk` in this session.
- **iOS/macOS**: `NativeBindings` calls `DynamicLibrary.process()`, which only works if the
  library is statically linked into the app binary - that linking (an Xcode build phase or CMake
  rule referencing `crates/ffi`'s `staticlib` output) doesn't exist yet. Needs a Mac to build and
  verify; this repository was developed on Windows.
- **Linux/Windows desktop Flutter apps**: no packaging exists yet either; use
  `libraryPathOverride` to point at your own build of `crates/ffi` in the meantime, the same way
  the test suite does.

## Running the tests yourself

```powershell
cargo build --release -p universal-payment-qr-ffi   # from the repository root
cd bindings/flutter
flutter pub get
flutter test
```
