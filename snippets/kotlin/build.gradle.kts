// :snippets:kotlin — the Kotlin examples the documentation site shows (ADR
// 30.9.26am D7). Every marked region is vendored at a released tag, and
// `make test-kotlin` compiles every file, so what the site shows compiled.
// install.kts sits outside every source set, so Gradle never compiles it.
// Ktor is the snippets' own dependency, as it would be the caller's.

plugins {
    id("lingara.kotlin-conventions")
}

dependencies {
    implementation(project(":kotlin"))
    implementation(libs.ktor.server.core)
    implementation(libs.ktor.server.cio)
}
