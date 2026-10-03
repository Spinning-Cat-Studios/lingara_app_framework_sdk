// :snippets:java — the Java examples the documentation site shows (ADR
// 30.9.26am D7). Every marked region is vendored at a released tag, and
// `make test-java` compiles every file, so what the site shows compiled.
// install.xml sits outside every source set, so Gradle never reads it.

plugins {
    id("lingara.java-conventions")
}

dependencies {
    implementation(project(":java"))
}
