// The repo-root Gradle build the Java and Kotlin kits share (ADR 30.9.26am
// D2, D8), and the openapi-generator runs Ruby and PHP read through :codegen.
//
// scs-snapshot reads this file statically to learn which directories the
// build needs, so it holds only literal include(...) calls and one
// includeBuild: no loop, and no project directory reassigned. Every project's
// directory is its path, `:java:conformance` being java/conformance/.

pluginManagement {
    includeBuild("build-logic")
    repositories {
        gradlePluginPortal()
        mavenCentral()
    }
}

dependencyResolutionManagement {
    repositories {
        mavenCentral()
    }
}

rootProject.name = "lingara-app-framework-sdk"

include(":codegen")
include(":java")
include(":java:conformance")
include(":snippets:java")
include(":kotlin")
include(":kotlin:conformance")
include(":snippets:kotlin")
