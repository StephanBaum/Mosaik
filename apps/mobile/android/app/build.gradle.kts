import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "io.github.stephanbaum.mosaic"
    compileSdk = 36

    defaultConfig {
        // The final application ID is OD-M10-1.
        applicationId = "io.github.stephanbaum.mosaic"
        minSdk = 28
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        // 64-bit only (OD-M10-9); matches the libraries scripts/mobile/build-android-native.sh builds.
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            // Unsigned: signing keys never exist in CI jobs without secrets (release is a later track).
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
    }

    packaging {
        resources.excludes += "/META-INF/{AL2.0,LGPL2.1}"
    }

    // The fonts (DM Sans, IBM Plex Mono; SIL OFL 1.1) are vendored once for both platforms and read as assets.
    sourceSets.getByName("main").assets.srcDir(rootDir.resolve("../shared/fonts"))

    testOptions.unitTests.isReturnDefaultValues = true

    lint {
        abortOnError = true
        warningsAsErrors = false
        checkReleaseBuilds = true
        // JNA mentions java.awt (absent on Android) in code that Android never loads.
        disable += "InvalidPackage"
    }
}

kotlin {
    jvmToolchain(21)
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
        allWarningsAsErrors.set(true)
    }
}

// JNA loads the host library of the core in JVM unit tests (the UniFFI library override); same as :core.
val hostLibrary: String =
    providers.gradleProperty("mosaic.hostLib").orNull
        ?: (System.getenv("CARGO_TARGET_DIR") ?: rootDir.resolve("../../../target").path)
            .let { "$it/debug/libmosaic_mobile_ffi.so" }

tasks.withType<Test>().configureEach {
    systemProperty("uniffi.component.mosaic.libraryOverride", hostLibrary)
}

dependencies {
    implementation(project(":core"))

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.ui)
    implementation(libs.compose.foundation)
    implementation(libs.compose.material3)
    implementation(libs.coroutines.android)
    debugImplementation(libs.compose.ui.tooling)

    testImplementation(libs.junit)
    testImplementation(libs.jna)

    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(libs.androidx.test.rules)
    // Compose UI tests (UnlockFlowTest, ScreenshotTest); test-only, so the release classpath is unchanged.
    androidTestImplementation(platform(libs.compose.bom))
    androidTestImplementation(libs.compose.ui.test.junit4)
    debugImplementation(libs.compose.ui.test.manifest)
}
