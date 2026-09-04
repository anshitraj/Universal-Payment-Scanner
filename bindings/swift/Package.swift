// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "UniversalPaymentQR",
    platforms: [.iOS(.v13), .macOS(.v11)],
    products: [
        .library(name: "UniversalPaymentQR", targets: ["UniversalPaymentQR"])
    ],
    targets: [
        // Declares the crates/ffi C ABI (see include/universal_payment_qr_ffi.h). The actual
        // implementation is NOT built by SwiftPM here - it comes from a prebuilt static library
        // this target links against once one exists. See README.md "Building" for why that step
        // needs a Mac and hasn't been run in this repository yet.
        .target(
            name: "CUniversalPaymentQR",
            linkerSettings: [
                .linkedLibrary("universal_payment_qr_ffi"),
                .unsafeFlags(["-LNativeLibraries"]),
            ]
        ),
        .target(
            name: "UniversalPaymentQR",
            dependencies: ["CUniversalPaymentQR"]
        ),
        .testTarget(
            name: "UniversalPaymentQRTests",
            dependencies: ["UniversalPaymentQR"]
        ),
    ]
)
