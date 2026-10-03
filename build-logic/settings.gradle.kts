// The included build that holds the JVM convention plugins (ADR 30.9.26am D8, after the clients' 29.9.26r D9).

dependencyResolutionManagement {
    repositories {
        gradlePluginPortal()
        mavenCentral()
    }
    versionCatalogs {
        create("libs") {
            from(files("../gradle/libs.versions.toml"))
        }
    }
}

rootProject.name = "build-logic"
