// :java:conformance — the Java kit's fixture app (ADR 30.9.26am D9; the
// contract's §F), on the kit's public API and JdkHttpHandler only. A separate
// project, so it is never in the published jar; the JDK's HttpServer is its
// server, so it declares nothing more. `installDist` writes the start script
// make/java.mk runs: build/install/conformance/bin/conformance.

plugins {
    id("lingara.java-conventions")
    application
}

dependencies {
    implementation(project(":java"))
}

application {
    mainClass = "com.getlingara.apps.conformance.Fixture"
}
