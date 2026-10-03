// Maven Central publishing in the clients' shape (their 29.9.26v D6a, ported
// by ADR 30.9.26am D8), applied by java/ and kotlin/.
//
// - com.vanniktech.maven.publish speaks the Central Portal API and signs from
//   in-memory keys, so no keyring file exists anywhere. The keys and the
//   Portal token arrive as ORG_GRADLE_PROJECT_signingInMemoryKey,
//   ...signingInMemoryKeyPassword, ...mavenCentralUsername and
//   ...mavenCentralPassword.
// - The artifact is `lingara-apps-<project name>`: lingara-apps-java,
//   lingara-apps-kotlin.
// - Java's javadoc jar is real javadoc output; Kotlin's is the plugin's empty
//   jar, since Central requires the file and not its contents.
// - The `staging` file repository under the root build/ is what staging's
//   dry run publishes to: only a repository publish writes the .md5 and .sha1
//   files Central requires, which publishToMavenLocal does not.

import com.vanniktech.maven.publish.JavaLibrary
import com.vanniktech.maven.publish.JavadocJar
import com.vanniktech.maven.publish.KotlinJvm

plugins {
    id("com.vanniktech.maven.publish")
}

mavenPublishing {
    publishToMavenCentral(automaticRelease = true)
    signAllPublications()
    coordinates("com.getlingara", "lingara-apps-${project.name}", project.version.toString())
    pom {
        name = "lingara-apps-${project.name}"
        description = "The official ${project.name.replaceFirstChar(Char::uppercase)} kit for Lingara apps."
        url = "https://github.com/Spinning-Cat-Studios/lingara_app_framework_sdk"
        licenses {
            license {
                name = "MIT"
                url = "https://opensource.org/license/mit"
            }
        }
        scm {
            url = "https://github.com/Spinning-Cat-Studios/lingara_app_framework_sdk"
            connection = "scm:git:https://github.com/Spinning-Cat-Studios/lingara_app_framework_sdk.git"
            developerConnection = "scm:git:ssh://git@github.com/Spinning-Cat-Studios/lingara_app_framework_sdk.git"
        }
        developers {
            developer {
                id = "spinning-cat-studios"
                name = "Spinning Cat Studios"
                email = "support@getlingara.com"
            }
        }
    }
}

pluginManager.withPlugin("org.jetbrains.kotlin.jvm") {
    mavenPublishing {
        configure(KotlinJvm(javadocJar = JavadocJar.Empty(), sourcesJar = true))
    }
}

pluginManager.withPlugin("java-library") {
    if (!pluginManager.hasPlugin("org.jetbrains.kotlin.jvm")) {
        mavenPublishing {
            configure(JavaLibrary(javadocJar = JavadocJar.Javadoc(), sourcesJar = true))
        }
    }
}

publishing {
    repositories {
        maven {
            name = "staging"
            url = uri(rootProject.layout.buildDirectory.dir("staging-repo"))
        }
    }
}
