# R8 keep rules of the app. JNA and the generated UniFFI bindings (package io.github.stephanbaum.mosaic.ffi,
# crates/mobile-ffi/uniffi.toml) are reached through native callbacks and reflection; the :core module
# ships the same rules as consumer rules, repeated here so the release check proves them.
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-keep class io.github.stephanbaum.mosaic.ffi.** { *; }
-keep class uniffi.** { *; }
-dontwarn java.awt.**
-dontwarn com.sun.jna.**
