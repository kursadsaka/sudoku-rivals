plugins { id("com.android.application"); id("org.jetbrains.kotlin.android"); id("org.jetbrains.kotlin.plugin.compose") }
android {
    namespace = "com.sudokurivals"
    compileSdk = 36
    buildToolsVersion = "36.0.0"
    ndkVersion = "28.2.13676358"
    defaultConfig {
        applicationId = "com.sudokurivals"; minSdk = 26; targetSdk = 36; versionCode = 1; versionName = "0.1"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
        val serverUrl = providers.gradleProperty("sudokuMatchServerUrl").orElse("").get()
        buildConfigField("String", "MATCH_SERVER_URL", "\"" + serverUrl.replace("\\", "\\\\").replace("\"", "\\\"") + "\"")
        buildConfigField("boolean", "STAGING", "false")
        manifestPlaceholders["appLabel"] = "Sudoku Rivals"
    }
    buildTypes {
        // Test-only HTTPS staging build. The origin is a fixed constant, not the
        // -PsudokuMatchServerUrl property, so no build input can point it elsewhere.
        // It starts from release (non-debuggable, no debug manifest: no cleartext or
        // loopback), has its own application ID so it never touches debug/release
        // records, and uses the local debug keystore only so it installs on emulators.
        // It is not a signed release and carries no App Links association.
        create("staging") {
            initWith(getByName("release"))
            matchingFallbacks += "release"
            applicationIdSuffix = ".staging"
            versionNameSuffix = "-staging"
            signingConfig = signingConfigs.getByName("debug")
            buildConfigField("String", "MATCH_SERVER_URL", "\"https://sudoku-rivals-g16-staging.onrender.com\"")
            buildConfigField("boolean", "STAGING", "true")
            manifestPlaceholders["appLabel"] = "Sudoku Rivals Staging"
        }
    }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    buildFeatures { compose = true; buildConfig = true }
}
dependencies {
    implementation("androidx.activity:activity-compose:1.10.0")
    implementation(platform("androidx.compose:compose-bom:2025.01.01"))
    implementation("androidx.compose.material3:material3")
    implementation("net.java.dev.jna:jna:5.17.0@aar")
}
