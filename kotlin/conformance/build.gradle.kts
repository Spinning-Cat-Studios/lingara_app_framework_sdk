// :kotlin:conformance — the Kotlin kit's fixture app (ADR 30.9.26am D9; the
// contract's §F), on the kit's public API and Route.lingaraApp only, served by
// Ktor 3 on the CIO engine. Both are conformance-only dependencies: this
// project is never published. `installDist` writes the start script
// make/kotlin.mk runs: build/install/conformance/bin/conformance.

plugins {
    id("lingara.kotlin-conventions")
    application
}

dependencies {
    implementation(project(":kotlin"))
    implementation(libs.ktor.server.core)
    implementation(libs.ktor.server.cio)
}

application {
    mainClass = "com.getlingara.apps.kotlin.conformance.FixtureKt"
}
