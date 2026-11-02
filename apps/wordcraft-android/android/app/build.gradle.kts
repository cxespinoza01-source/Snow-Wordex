plugins {
    id("com.android.application")
}

val repositoryRoot = rootProject.projectDir.resolve("../../..").canonicalFile
val rustLibraryOutput = projectDir.resolve("src/main/jniLibs")

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
