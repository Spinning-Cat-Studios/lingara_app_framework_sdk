// lingara-apps-kotlin: the Kotlin app kit (ADR 30.9.26am D2).
//
// Generated app types live in src/generated/kotlin (committed, and written
// only by `make codegen-kotlin`); the hand-written core in src/main/kotlin.
// Both compile in `main`, so explicit-API mode and allWarningsAsErrors cover
// the generated code too. The one runtime dependency is the Lingara library,
// written literally on the line below so tools/library-floor/check.sh can
// read the floor (LIBRARY_FLOOR); kotlinx-serialization and coroutines come
// through it. Ktor 3 is compileOnly: Route.lingaraApp needs it, and a caller
// without Ktor never loads that file, so no io.ktor artifact reaches the POM.

plugins {
    id("lingara.kotlin-conventions")
    id("lingara.publishing-conventions")
    kotlin("plugin.serialization")
}

description = "The official Kotlin kit for Lingara apps."

kotlin {
    explicitApi()
    sourceSets.named("main") {
        kotlin.srcDir("src/generated/kotlin")
    }
}

dependencies {
    api("com.getlingara:lingara-kotlin:0.1.0-alpha.10")
    compileOnly(libs.ktor.server.core)
    testImplementation(libs.ktor.server.test.host)
}

val pom = tasks.named("generatePomFileForMavenPublication")

tasks.test {
    dependsOn(pom)
    systemProperty("lingara.cardVectors", rootProject.file("conformance/vectors/card-limits.json").path)
    systemProperty("lingara.manifestVectors", rootProject.file("conformance/vectors/manifest.json").path)
    systemProperty("lingara.pom", layout.buildDirectory.file("publications/maven/pom-default.xml").get().asFile.path)
}
