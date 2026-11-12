import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
}

// The UniFFI Kotlin bindings are generated, never committed: scripts/mobile/bindgen.sh writes them here.
// CI runs the script before Gradle; locally:
//   bash scripts/mobile/bindgen.sh --out apps/mobile/android/core/build/generated/uniffi kotlin
val uniffiOut = layout.buildDirectory.dir("generated/uniffi/kotlin")

android {
    namespace = "io.github.stephanbaum.mosaic.core"
    compileSdk = 36

    defaultConfig {
        minSdk = 28
        consumerProguardFiles("consumer-rules.pro")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets.getByName("main").java.srcDir(uniffiOut)

    testOptions.unitTests.isReturnDefaultValues = true
}

kotlin {
    jvmToolchain(21)
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
        // Deviation from "allWarningsAsErrors everywhere": this module compiles the generated UniFFI
        // code, whose warnings we do not control. The hand-written code here is kept warning-free.
        allWarningsAsErrors.set(false)
    }
}

val checkUniffiBindings by tasks.registering {
    val dir = uniffiOut
    doLast {
        if (!dir.get().asFile.walkTopDown().any { it.isFile && it.extension == "kt" }) {
            throw GradleException(
                "No UniFFI bindings in ${dir.get().asFile}. Run: " +
                    "bash scripts/mobile/bindgen.sh --out apps/mobile/android/core/build/generated/uniffi kotlin",
            )
        }
    }
}
tasks.named("preBuild") { dependsOn(checkUniffiBindings) }

// JNA loads the host library of the core in JVM unit tests (the UniFFI library override).
val hostLibrary: String =
    providers.gradleProperty("mosaic.hostLib").orNull
        ?: (System.getenv("CARGO_TARGET_DIR") ?: rootDir.resolve("../../../target").path)
            .let { "$it/debug/libmosaic_mobile_ffi.so" }

val workspaceVersion: String =
    Regex("""(?m)^version\s*=\s*"([^"]+)"""")
        .find(rootDir.resolve("../../../Cargo.toml").readText())!!
        .groupValues[1]

tasks.withType<Test>().configureEach {
    systemProperty("uniffi.component.mosaic.libraryOverride", hostLibrary)
    systemProperty("mosaic.expectedVersion", workspaceVersion)
}

dependencies {
    // api: the generated bindings expose JNA types (Pointer) to :app, which must compile against them.
    api("net.java.dev.jna:jna:${libs.versions.jna.get()}@aar")
    api(libs.coroutines.android)

    testImplementation(libs.junit)
    testImplementation(libs.jna)
}
