// Transcribed from status-report/03-sdk-bindings.md. "Verified" means a real build ran and a real
// test suite passed in this repo's own dev session — not "should work". Kept honest on purpose:
// not every row is green.
export interface SdkStatus {
  name: string;
  path: string;
  status: "verified" | "partial" | "unverified";
  note: string;
}

export const SDK_STATUSES: readonly SdkStatus[] = [
  { name: "Rust", path: "crates/core", status: "verified", note: "66 unit tests, clippy-clean, fuzzed" },
  { name: "JavaScript / Node", path: "packages/core", status: "verified", note: "WASM, live in this playground" },
  { name: "Python", path: "bindings/python", status: "verified", note: "10/10 pytest via maturin" },
  { name: "Android / Kotlin", path: "bindings/android", status: "verified", note: "9/9 JVM tests, real .aar" },
  { name: "Flutter", path: "bindings/flutter", status: "partial", note: "8/8 flutter test — mobile packaging (apk/ios build) unverified" },
  { name: "Swift / iOS", path: "bindings/swift", status: "unverified", note: "Written, never compiled — needs Xcode/macOS" },
  { name: "React Native", path: "bindings/react-native", status: "unverified", note: "Interface only — every call throws until native modules are wired" },
  { name: "Go (hosted API)", path: "services/api-go", status: "verified", note: "Delegates to the verified Rust CLI" },
];
