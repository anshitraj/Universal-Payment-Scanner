# universal-payment-qr (Android / Kotlin)

JNI bindings over the same Rust core every SDK in this project uses (`crates/core`) - no parsing
logic is reimplemented here. Verified end to end on this repository: cross-compiled for
arm64-v8a/armeabi-v7a/x86_64, 9/9 Kotlin unit tests pass against a real build, and `assembleRelease`
produces a real `.aar`.

```kotlin
import org.universalpaymentqr.UniversalPaymentQR

val result = UniversalPaymentQR.parse("upi://pay?pa=merchant%40bank&am=499.00&cu=INR")
result.getString("scheme") // "upi"
```

```kotlin
import org.universalpaymentqr.createScanner

val scanner = createScanner(schemes = mapOf("upi" to true, "bitcoin" to false))
scanner.scan(payload)
```

## Building

The compiled `.so` files under `lib/src/main/jniLibs/` are build output (gitignored), not
checked in - same reasoning as this repo never commits `target/`. Produce them first:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk
export ANDROID_NDK_HOME=/path/to/Android/Sdk/ndk/<version>
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 \
  -o bindings/android/lib/src/main/jniLibs \
  build --release -p universal_payment_qr_android
```

Then, from `bindings/android/`:

```bash
./gradlew :lib:assembleRelease   # -> lib/build/outputs/aar/lib-release.aar
```

## Local unit tests

`./gradlew test` runs on the host JVM, not a device, so it can't load the Android-targeted `.so`
files above. It loads a host-native build of the same JNI crate instead:

```bash
cargo build --release -p universal_payment_qr_android
cp ../../target/release/universal_payment_qr_android.dll lib/native-test-libs/   # .so/.dylib on Linux/macOS
./gradlew :lib:testDebugUnitTest
```

(`native-test-libs/` is gitignored, dev-machine-only - the `java.library.path` wiring for it is
in `lib/build.gradle.kts`.) Android's `org.json` is a platform API with stub-only classes on the
host JVM; `testImplementation("org.json:json:...")` in `build.gradle.kts` provides a real
implementation so the unit tests actually exercise JSON parsing rather than silently getting nulls
- this is the one thing in this binding that isn't obvious from the code alone.

## What wasn't verified here

Instrumented tests (`./gradlew connectedAndroidTest`, needs a device/emulator) and Maven
publishing weren't run in this session - the JVM unit tests above are real, but they exercise the
JNI bridge on a host build, not the actual arm64/armv7/x86_64 `.so` files on a device.
