plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    `maven-publish`
}

android {
    namespace = "org.universalpaymentqr"
    compileSdk = 35

    defaultConfig {
        minSdk = 21 // arm64-v8a/x86_64 need API 21+; armeabi-v7a is included down to this floor too.
        ndk {
            // Must match the ABIs cross-compiled into src/main/jniLibs (see bindings/android/README.md).
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }
        consumerProguardFiles("consumer-rules.pro")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }

}

dependencies {
    testImplementation("junit:junit:4.13.2")
    // org.json is an Android *platform* API: on a real device it's provided by the OS, so it's
    // `compileOnly`-equivalent in production (never bundled). On the host JVM that runs local unit
    // tests, android.jar only ships stub classes that throw (or, worse, silently return null/0/
    // false if `isReturnDefaultValues` is set) - this is a real, independently-published
    // implementation of the same `org.json` package, for tests only, so JSONObject/JSONArray
    // actually work when running `./gradlew test`.
    testImplementation("org.json:json:20240303")
}

// Local JVM unit tests (./gradlew test) run on the host JVM, not an Android device, so they can't
// load the Android-targeted .so files under src/main/jniLibs. They load a host-native build of
// the same JNI crate instead - see README.md "Local unit tests" for how to produce it.
tasks.withType<Test> {
    systemProperty("java.library.path", file("native-test-libs").absolutePath)
}

publishing {
    publications {
        register<MavenPublication>("release") {
            groupId = "org.universalpaymentqr"
            artifactId = "universal-payment-qr"
            version = "0.1.0"
            afterEvaluate { from(components["release"]) }
        }
    }
}
