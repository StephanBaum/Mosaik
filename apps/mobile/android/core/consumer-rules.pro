# JNA and the generated UniFFI bindings are reached by reflection and native callbacks.
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-keep class io.github.stephanbaum.mosaic.ffi.** { *; }
-dontwarn java.awt.**
-dontwarn com.sun.jna.**
