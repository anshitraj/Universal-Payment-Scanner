/// Deterministic, local-first payment QR/address parsing - UPI, Pix, EMVCo, Bitcoin, Ethereum,
/// and more, one normalized result. Backed by the same Rust core every SDK in this project uses
/// (`crates/core`), via `dart:ffi` - no parsing logic is reimplemented here. No network calls, no
/// credentials, no payment execution.
///
/// ```dart
/// final result = UniversalPaymentQr.parse("upi://pay?pa=merchant%40bank&am=499.00&cu=INR");
/// print(result['scheme']); // upi
/// ```
library unipayscan;

import 'dart:convert';
import 'dart:ffi';

import 'package:ffi/ffi.dart';

import 'src/native_bindings.dart';

String? _policyJson(Map<String, dynamic>? policy) => policy == null ? null : jsonEncode(policy);

Pointer<Utf8> _toNative(String value) => value.toNativeUtf8();

Map<String, dynamic> _decode(Pointer<Utf8> resultPtr, NativeBindings bindings) {
  try {
    final jsonText = resultPtr.toDartString();
    return jsonDecode(jsonText) as Map<String, dynamic>;
  } finally {
    bindings.freeString(resultPtr);
  }
}

List<dynamic> _decodeList(Pointer<Utf8> resultPtr, NativeBindings bindings) {
  try {
    final jsonText = resultPtr.toDartString();
    return jsonDecode(jsonText) as List<dynamic>;
  } finally {
    bindings.freeString(resultPtr);
  }
}

/// Static entry points using a lazily-opened default native library. For tests, or until this
/// binding's iOS/desktop packaging exists (see `native_bindings.dart`), use [UniversalPaymentQr]
/// with an explicit `libraryPathOverride` instead.
class UniversalPaymentQr {
  static NativeBindings _bindings({String? libraryPathOverride}) =>
      NativeBindings.instance(libraryPathOverride: libraryPathOverride);

  /// Parses [payload] (a string already decoded from a QR image, or pasted/typed directly) with
  /// the default policy, or with [policy] (matching `CapabilityPolicy`'s JSON shape - see
  /// `crates/core/src/policy.rs`) if given. Returns a normalized PaymentIntent map - see
  /// docs/schema.md in the repository.
  static Map<String, dynamic> parse(
    String payload, {
    Map<String, dynamic>? policy,
    String? libraryPathOverride,
  }) {
    final bindings = _bindings(libraryPathOverride: libraryPathOverride);
    final payloadPtr = _toNative(payload);
    final policyText = _policyJson(policy);
    final policyPtr = policyText == null ? nullptr : _toNative(policyText);
    try {
      final result = bindings.parse(payloadPtr, policyPtr);
      return _decode(result, bindings);
    } finally {
      malloc.free(payloadPtr);
      if (policyPtr != nullptr) malloc.free(policyPtr);
    }
  }

  /// Cheap detection only (scheme id + confidence), without full parsing/validation.
  static Map<String, dynamic> detect(
    String payload, {
    Map<String, dynamic>? policy,
    String? libraryPathOverride,
  }) {
    final bindings = _bindings(libraryPathOverride: libraryPathOverride);
    final payloadPtr = _toNative(payload);
    final policyText = _policyJson(policy);
    final policyPtr = policyText == null ? nullptr : _toNative(policyText);
    try {
      final result = bindings.detect(payloadPtr, policyPtr);
      return _decode(result, bindings);
    } finally {
      malloc.free(payloadPtr);
      if (policyPtr != nullptr) malloc.free(policyPtr);
    }
  }

  /// Every registered scheme's metadata (id, maturity, features, references, ...), independent of
  /// any capability policy. Use this to build a scheme-toggle UI.
  static List<dynamic> capabilities({String? libraryPathOverride}) {
    final bindings = _bindings(libraryPathOverride: libraryPathOverride);
    final result = bindings.schemes(nullptr);
    return _decodeList(result, bindings);
  }

  /// [capabilities] filtered to schemes at `stable` or `beta` maturity.
  static List<dynamic> supportedSchemes({String? libraryPathOverride}) => capabilities(
        libraryPathOverride: libraryPathOverride,
      ).where((scheme) => (scheme as Map<String, dynamic>)['maturity'] != 'experimental').toList();

  /// The crate version backing this build (`CARGO_PKG_VERSION`), kept in one place.
  static String version({String? libraryPathOverride}) {
    final bindings = _bindings(libraryPathOverride: libraryPathOverride);
    final result = bindings.version();
    try {
      return result.toDartString();
    } finally {
      bindings.freeString(result);
    }
  }
}

/// A parser bound to one fixed capability policy - construct via `createScanner()`, not directly.
class Scanner {
  Scanner._(this._policy, this._libraryPathOverride);

  final Map<String, dynamic>? _policy;
  final String? _libraryPathOverride;

  Map<String, dynamic> scan(String payload) =>
      UniversalPaymentQr.parse(payload, policy: _policy, libraryPathOverride: _libraryPathOverride);

  Map<String, dynamic> detectPayload(String payload) => UniversalPaymentQr.detect(
        payload,
        policy: _policy,
        libraryPathOverride: _libraryPathOverride,
      );

  List<dynamic> schemes() =>
      UniversalPaymentQr.capabilities(libraryPathOverride: _libraryPathOverride);
}

/// Builds a [Scanner] bound to a capability policy. All arguments are optional and match the
/// JS/Python/Kotlin SDKs' `createScanner({...})` shape - e.g. `preset: "india"`, or
/// `schemes: {"upi": true, "bitcoin": false}`. Detection always stays global; this only controls
/// what your application accepts as `supported`.
Scanner createScanner({
  String? preset,
  Map<String, bool>? schemes,
  Map<String, bool>? categories,
  Map<String, Map<String, bool>>? countries,
  String? libraryPathOverride,
}) {
  final policy = <String, dynamic>{};
  if (preset != null) policy['preset'] = preset;
  if (schemes != null) policy['schemes'] = schemes;
  if (categories != null) policy['categories'] = categories;
  if (countries != null) policy['countries'] = countries;
  return Scanner._(policy.isEmpty ? null : policy, libraryPathOverride);
}
