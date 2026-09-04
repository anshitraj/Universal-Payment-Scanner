# NativeBridge's method names and signatures must match the compiled .so's JNI symbol names
# exactly (Java_org_universalpaymentqr_NativeBridge_*) - R8/ProGuard must not rename or strip it.
-keep class org.universalpaymentqr.NativeBridge { *; }
