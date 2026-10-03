// The convention plugins' own build (ADR 30.9.26am D8, after the clients' 29.9.26r D9). Each plugin a
// convention applies is on this classpath at its catalogue pin, so no project
// build file names a plugin version.

plugins {
    `kotlin-dsl`
}

dependencies {
    implementation(libs.spotless.plugin)
    // The Maven Central publishing plugin.
    implementation(libs.maven.publish.plugin)
    // lingara.kotlin-conventions applies the first, and
    // kotlin/build.gradle.kts the second, each without a version.
    implementation(libs.kotlin.gradle.plugin)
    implementation(libs.kotlin.serialization.plugin)
}
