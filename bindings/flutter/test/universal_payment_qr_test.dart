import 'package:test/test.dart';
import 'package:universal_payment_qr/universal_payment_qr.dart';

// Points at a host build of crates/ffi (see bindings/flutter/README.md "Running tests") -
// production Android use resolves the bundled .so via NativeBindings' default Android branch
// instead; this override exists so desktop `dart test` can exercise the same FFI contract without
// a device/emulator.
const _libraryPathOverride =
    String.fromEnvironment('UPQR_NATIVE_LIB', defaultValue: '../../target/release/universal_payment_qr_ffi.dll');

void main() {
  test('parses UPI without floating point drift', () {
    final result = UniversalPaymentQr.parse(
      'upi://pay?pa=merchant%40bank&pn=Example&am=499.00&cu=INR',
      libraryPathOverride: _libraryPathOverride,
    );
    expect(result['recognized'], isTrue);
    expect(result['scheme'], 'upi');
    expect(result['amount'], '499.00');
  });

  test('recognizes a bare bitcoin address with no uri scheme', () {
    final result = UniversalPaymentQr.parse(
      '1BoatSLRHtKNngkdXEeobR76b53LETtpyT',
      libraryPathOverride: _libraryPathOverride,
    );
    expect(result['recognized'], isTrue);
    expect(result['scheme'], 'bitcoin');
  });

  test('non-payment QR is recognized but flagged, not left fully unrecognized', () {
    final result = UniversalPaymentQr.parse(
      'https://example.com/docs',
      libraryPathOverride: _libraryPathOverride,
    );
    expect(result['recognized'], isTrue);
    expect(result['scheme'], 'url');
    expect(result['supported'], isFalse);
    expect((result['support'] as Map)['reason'], 'NOT_PAYMENT_QR');
  });

  test('disabled scheme stays recognized but unsupported', () {
    final scanner = createScanner(
      schemes: {'solana_pay': false},
      libraryPathOverride: _libraryPathOverride,
    );
    final result = scanner.scan('solana:9xQeWvG816bUx9EPfEZi4q3G44E2sA6v7a6D9k5GmRse?amount=1.25');
    expect(result['recognized'], isTrue);
    expect(result['supported'], isFalse);
    expect((result['support'] as Map)['reason'], 'SCHEME_DISABLED');
  });

  test('preset scopes to india only', () {
    final scanner = createScanner(preset: 'india', libraryPathOverride: _libraryPathOverride);
    final upi = scanner.scan('upi://pay?pa=merchant%40bank&am=1&cu=INR');
    expect(upi['supported'], isTrue);
    final bitcoin = scanner.scan('1BoatSLRHtKNngkdXEeobR76b53LETtpyT');
    expect(bitcoin['recognized'], isTrue);
    expect(bitcoin['supported'], isFalse);
  });

  test('capabilities lists every registered scheme', () {
    final capabilities = UniversalPaymentQr.capabilities(libraryPathOverride: _libraryPathOverride);
    final ids = capabilities.map((s) => (s as Map)['id']).toSet();
    expect(ids, contains('upi'));
    expect(ids, contains('pix'));
    expect(capabilities.length, greaterThan(30));
  });

  test('supported schemes excludes experimental', () {
    final supported = UniversalPaymentQr.supportedSchemes(libraryPathOverride: _libraryPathOverride);
    for (final scheme in supported) {
      expect((scheme as Map)['maturity'], isNot('experimental'));
    }
  });

  test('version is non-empty', () {
    expect(UniversalPaymentQr.version(libraryPathOverride: _libraryPathOverride), isNotEmpty);
  });
}
