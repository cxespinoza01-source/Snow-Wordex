plugins {
    id("com.android.application")
}

val repositoryRoot = rootProject.projectDir.resolve("../../..").canonicalFile
val rustLibraryOutput = projectDir.resolve("src/main/jniLibs")

// Pass the Android platform JAR directly to Cargo's build scripts.
// Gradle's Exec task does not rely on a shell profile to provide this.
val androidSdkRoot = System.getenv("ANDROID_HOME")
    ?: System.getenv("ANDROID_SDK_ROOT")
val androidJar = System.getenv("ANDROID_JAR")
    ?: androidSdkRoot?.let { "$it/platforms/android-35/android.jar" }

android {
    namespace = "org.wordcraft.android"
    compileSdk = 35

    defaultConfig {
        applicationId = "org.wordcraft.android"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        ndk {
            abiFilters += listOf("arm64-v8a")
        }
    }

    sourceSets["main"].manifest.srcFile("../AndroidManifest.xml")
}

// Kotlin 1.8+ folds the JDK 7/8 extensions into kotlin-stdlib.
// Older transitive jdk7/jdk8 artifacts otherwise duplicate those classes.
configurations.configureEach {
    exclude(group = "org.jetbrains.kotlin", module = "kotlin-stdlib-jdk7")
    exclude(group = "org.jetbrains.kotlin", module = "kotlin-stdlib-jdk8")
}

dependencies {
    // GameActivity's Java API references AppCompatActivity and AndroidX Core.
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.core:core:1.15.0")
    implementation("androidx.games:games-activity:4.4.0")
}

val buildRustAndroid by tasks.registering(Exec::class) {
    group = "build"
    description = "Cross-compiles the WordCraft native Rust library for Android ARM64."
    workingDir = repositoryRoot
    if (androidSdkRoot != null) {
        environment("ANDROID_HOME", androidSdkRoot)
        environment("ANDROID_SDK_ROOT", androidSdkRoot)
    }
    if (androidJar != null) {
        environment("ANDROID_JAR", androidJar)
    }
    commandLine(
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "-P", "26",
        "-o", rustLibraryOutput.absolutePath,
        "build", "--release", "-p", "wordcraft-android", "--lib"
    )
    inputs.files(
        fileTree(repositoryRoot.resolve("apps/wordcraft-android/src")),
        fileTree(repositoryRoot.resolve("crates")),
        repositoryRoot.resolve("apps/wordcraft-android/Cargo.toml"),
        repositoryRoot.resolve("Cargo.toml"),
        repositoryRoot.resolve("Cargo.lock")
    )
    outputs.dir(rustLibraryOutput)
}

tasks.named("preBuild").configure {
    dependsOn(buildRustAndroid)
}
