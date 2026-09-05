/// Raw `dart:ffi` signatures matching `crates/ffi/src/lib.rs` exactly. Every function returns an
/// owned, heap-allocated, NUL-terminated UTF-8 string that MUST be passed to `upqr_free_string`
/// exactly once - see that crate's module docs for the full memory contract. Not part of the
/// public API; [UniversalPaymentQr] in `universal_payment_scanner.dart` is.
library;

import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart' show Utf8;

typedef _UpqrParseNative = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef _UpqrParseDart = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);

typedef _UpqrSchemesNative = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _UpqrSchemesDart = Pointer<Utf8> Function(Pointer<Utf8>);

typedef _UpqrVersionNative = Pointer<Utf8> Function();
typedef _UpqrVersionDart = Pointer<Utf8> Function();

typedef _UpqrFreeStringNative = Void Function(Pointer<Utf8>);
typedef _UpqrFreeStringDart = void Function(Pointer<Utf8>);

/// Opens the native library and resolves every `upqr_*` symbol once. On Android the `.so` is
/// bundled by `android/build.gradle`'s `jniLibs.srcDirs`, so a bare name resolves correctly at
/// runtime. Other platforms need their own packaging (a Podspec for iOS/macOS, a CMake rule for
/// Linux/Windows desktop) that this binding doesn't set up yet - [libraryPathOverride] exists
/// specifically so tests (and, until that packaging exists, desktop apps) can point at an
/// explicit build of `crates/ffi` instead.
class NativeBindings {
  NativeBindings._(DynamicLibrary lib)
      : parse = lib.lookupFunction<_UpqrParseNative, _UpqrParseDart>('upqr_parse'),
        detect = lib.lookupFunction<_UpqrParseNative, _UpqrParseDart>('upqr_detect'),
        schemes = lib.lookupFunction<_UpqrSchemesNative, _UpqrSchemesDart>('upqr_schemes'),
        version = lib.lookupFunction<_UpqrVersionNative, _UpqrVersionDart>('upqr_version'),
        freeString =
            lib.lookupFunction<_UpqrFreeStringNative, _UpqrFreeStringDart>('upqr_free_string');

  final _UpqrParseDart parse;
  final _UpqrParseDart detect;
  final _UpqrSchemesDart schemes;
  final _UpqrVersionDart version;
  final _UpqrFreeStringDart freeString;

  static NativeBindings? _instance;

  factory NativeBindings.instance({String? libraryPathOverride}) {
    return _instance ??= NativeBindings._(_open(libraryPathOverride));
  }

  static DynamicLibrary _open(String? override) {
    if (override != null) return DynamicLibrary.open(override);
    if (Platform.isAndroid) return DynamicLibrary.open('libuniversal_payment_qr_ffi.so');
    if (Platform.isIOS || Platform.isMacOS) return DynamicLibrary.process();
    throw UnsupportedError(
      'No bundled native library for this platform yet. Pass libraryPathOverride to '
      'NativeBindings.instance() with a path to a build of crates/ffi - see '
      'bindings/flutter/README.md.',
    );
  }
}
