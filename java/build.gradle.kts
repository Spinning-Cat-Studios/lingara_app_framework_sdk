// lingara-apps-java: the Java app kit (ADR 30.9.26am D2).
//
// Generated app types live in src/generated/ (committed, and written only by
// `make codegen-java`); the hand-written core in src/main/. The one runtime
// dependency is the Lingara library, written literally on the line below so
// tools/library-floor/check.sh can read the floor (LIBRARY_FLOOR): Jackson
// and the signature verifier come through it. It is `api` because the kit's
// types carry Jackson's annotations and take its JsonNode. Publishing is the
// Maven Central convention plugin, whose artifact is
// lingara-apps-<project name>: lingara-apps-java.

plugins {
    id("lingara.java-conventions")
    id("lingara.publishing-conventions")
}

description = "The official Java kit for Lingara apps."

sourceSets {
    main {
        java.srcDir("src/generated/java")
    }
}

dependencies {
    api("com.getlingara:lingara-java:0.1.0-alpha.10")
}

val pom = tasks.named("generatePomFileForMavenPublication")

tasks.test {
    dependsOn(pom)
    systemProperty("lingara.cardVectors", rootProject.file("conformance/vectors/card-limits.json").path)
    systemProperty("lingara.manifestVectors", rootProject.file("conformance/vectors/manifest.json").path)
    systemProperty("lingara.pom", layout.buildDirectory.file("publications/maven/pom-default.xml").get().asFile.path)
    systemProperty("lingara.moduleInfo", layout.buildDirectory.file("classes/java/main/module-info.class").get().asFile.path)
}
