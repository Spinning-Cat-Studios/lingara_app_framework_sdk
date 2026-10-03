// :codegen — the generated half of the Java, Kotlin, Ruby and PHP kits
// (ADR 30.9.26am D6), in the clients' shape. openapi-generator is pinned
// once, in the catalogue, and reads the app view's 3.0 dialect.
//
// generateJava and generateKotlin each start from an empty staging
// directory, run openapi-generator's models there, then AppsCodegen (the
// three sealed unions and their arms, the ContextSlice kinds and the
// operations), and end with a Sync of the staged package into the kit's
// src/generated/. Starting empty is what lets AppsCodegen's refusal tell an
// openapi-generator copy of a union or an arm from its own last output; Sync
// deletes a committed file the view no longer produces.
//
// generateRuby and generatePhp are openapi-generator's models only; the
// unions, arms and operations are each kit's own emitter's (ruby/codegen,
// php/codegen), which make codegen-<lang> runs next.
//
// `-PcodegenOut=<dir>` changes only the sync target of whichever task runs:
// check-codegen-<lang> uses it to compare a fresh run with the committed tree.

import org.openapitools.generator.gradle.plugin.tasks.GenerateTask

plugins {
    id("lingara.java-conventions")
    alias(libs.plugins.openapi.generator)
}

dependencies {
    implementation(libs.jackson.databind)
}

val view = rootProject.file("spec/generator/apps.3.0.json")
val codegenOut = providers.gradleProperty("codegenOut").map { File(it) }

/** The view's `x-lingara-unions`, each mapped to its sealed type in [pkg]. */
fun unionMappings(pkg: String): Map<String, String> =
    listOf("CardElement", "ListItem", "ContextSlice").associateWith { "$pkg.$it" }

// ── Java ─────────────────────────────────────────────────────────────────────

val javaStaging = layout.buildDirectory.dir("java-staging")
val javaStagedSources = javaStaging.map { it.dir("openapi/src/main/java") }
val javaOut = codegenOut.orElse(rootProject.file("java/src/generated/java"))

val cleanJavaStaging = tasks.register<Delete>("cleanJavaStaging") {
    delete(javaStaging)
}

val generateJavaModels = tasks.register<GenerateTask>("generateJavaModels") {
    dependsOn(cleanJavaStaging)
    outputs.upToDateWhen { false }
    generatorName.set("java")
    library.set("native")
    inputSpec.set(view.path)
    outputDir.set(javaStaging.get().dir("openapi").asFile.path)
    modelPackage.set("com.getlingara.apps.model")
    ignoreFileOverride.set(file("java.openapi-generator-ignore").path)
    // Empty overrides of the javax nullability and @Generated annotations.
    templateDir.set(file("templates/java").path)
    globalProperties.set(mapOf("models" to "", "modelTests" to "false", "modelDocs" to "false"))
    configOptions.set(
        mapOf(
            "serializationLibrary" to "jackson",
            "openApiNullable" to "false",
            "hideGenerationTimestamp" to "true",
            "annotationLibrary" to "none",
            "sourceFolder" to "src/main/java",
        ),
    )
    // As the library: no unsigned types in Java, and ids stay strings.
    typeMappings.set(
        mapOf(
            "integer+uint32" to "Long",
            "integer+uint8" to "Integer",
            "string+date-time" to "String",
            "string+uuid" to "String",
        ),
    )
    additionalProperties.set(mapOf("supportUrlQuery" to false))
    // The three unions are bare `{type: object}` schemas in the view, which
    // the pin types as Object; mapped, a reference to one names the sealed
    // interface AppsCodegen writes into the same package.
    schemaMappings.set(unionMappings("com.getlingara.apps.model"))
}

val generateJavaApps = tasks.register<JavaExec>("generateJavaApps") {
    dependsOn(generateJavaModels)
    outputs.upToDateWhen { false }
    classpath = sourceSets.main.get().runtimeClasspath
    mainClass = "com.getlingara.codegen.AppsCodegen"
    args(view.path, javaStagedSources.get().asFile.path)
}

tasks.register<Sync>("generateJava") {
    dependsOn(generateJavaApps)
    outputs.upToDateWhen { false }
    from(javaStagedSources) {
        include("com/getlingara/apps/**")
    }
    into(javaOut)
}

// ── Kotlin ───────────────────────────────────────────────────────────────────

val kotlinStaging = layout.buildDirectory.dir("kotlin-staging")
val kotlinStagedSources = kotlinStaging.map { it.dir("openapi/src/main/kotlin") }
val kotlinOut = codegenOut.orElse(rootProject.file("kotlin/src/generated/kotlin"))

val cleanKotlinStaging = tasks.register<Delete>("cleanKotlinStaging") {
    delete(kotlinStaging)
}

val generateKotlinModels = tasks.register<GenerateTask>("generateKotlinModels") {
    dependsOn(cleanKotlinStaging)
    outputs.upToDateWhen { false }
    generatorName.set("kotlin")
    inputSpec.set(view.path)
    outputDir.set(kotlinStaging.get().dir("openapi").asFile.path)
    modelPackage.set("com.getlingara.apps.kotlin.model")
    ignoreFileOverride.set(file("kotlin.openapi-generator-ignore").path)
    // The two property templates without @Contextual, as the library's.
    templateDir.set(file("templates/kotlin").path)
    globalProperties.set(mapOf("models" to "", "modelTests" to "false", "modelDocs" to "false"))
    configOptions.set(
        mapOf(
            "serializationLibrary" to "kotlinx_serialization",
            "explicitApi" to "true",
            "enumPropertyNaming" to "UPPERCASE",
            "sourceFolder" to "src/main/kotlin",
        ),
    )
    typeMappings.set(
        mapOf(
            "integer+uint32" to "kotlin.Long",
            "integer+uint8" to "kotlin.Int",
            "string+date-time" to "kotlin.String",
            "string+uuid" to "kotlin.String",
        ),
    )
    schemaMappings.set(unionMappings("com.getlingara.apps.kotlin.model"))
}

val generateKotlinApps = tasks.register<JavaExec>("generateKotlinApps") {
    dependsOn(generateKotlinModels)
    outputs.upToDateWhen { false }
    classpath = sourceSets.main.get().runtimeClasspath
    mainClass = "com.getlingara.codegen.AppsCodegen"
    args("--kotlin", view.path, kotlinStagedSources.get().asFile.path)
}

tasks.register<Sync>("generateKotlin") {
    dependsOn(generateKotlinApps)
    outputs.upToDateWhen { false }
    from(kotlinStagedSources) {
        include("com/getlingara/apps/kotlin/**")
    }
    into(kotlinOut)
}

// ── Ruby ─────────────────────────────────────────────────────────────────────
//
// openapi-generator's `ruby` models, module Lingara::Apps, synced from the
// staged lib/lingara-apps/models/ into ruby/lib/lingara/apps/generated/.
// The unions, their arms and the operations are ruby/codegen/generate.rb's.

val rubyStaging = layout.buildDirectory.dir("ruby-staging")
val rubyOut = codegenOut.orElse(rootProject.file("ruby/lib/lingara/apps/generated"))

val cleanRubyStaging = tasks.register<Delete>("cleanRubyStaging") {
    delete(rubyStaging)
}

val generateRubyModels = tasks.register<GenerateTask>("generateRubyModels") {
    dependsOn(cleanRubyStaging)
    outputs.upToDateWhen { false }
    generatorName.set("ruby")
    inputSpec.set(view.path)
    outputDir.set(rubyStaging.get().asFile.path)
    ignoreFileOverride.set(file("ruby.openapi-generator-ignore").path)
    globalProperties.set(mapOf("models" to "", "modelTests" to "false", "modelDocs" to "false"))
    configOptions.set(
        mapOf(
            "moduleName" to "Lingara::Apps",
            "gemName" to "lingara-apps",
            "hideGenerationTimestamp" to "true",
            "enumUnknownDefaultCase" to "true",
        ),
    )
}

tasks.register<Sync>("generateRuby") {
    dependsOn(generateRubyModels)
    outputs.upToDateWhen { false }
    from(rubyStaging.map { it.dir("lib/lingara-apps/models") })
    into(rubyOut)
}

// ── PHP ──────────────────────────────────────────────────────────────────────
//
// openapi-generator's `php-nextgen` models, namespace Lingara\Apps\Generated,
// plus the two supporting files they reference: src/Generated/** (models and
// ModelInterface.php) and src/ObjectSerializer.php, synced into php/src/.
// Every other file under php/src/ is preserved, so the hand-written core and
// php/codegen/generate.php's outputs survive the Sync. The unions, their
// arms and the operations are generate.php's.

val phpStaging = layout.buildDirectory.dir("php-staging")
val phpOut = codegenOut.orElse(rootProject.file("php/src"))

val cleanPhpStaging = tasks.register<Delete>("cleanPhpStaging") {
    delete(phpStaging)
}

val generatePhpModels = tasks.register<GenerateTask>("generatePhpModels") {
    dependsOn(cleanPhpStaging)
    outputs.upToDateWhen { false }
    generatorName.set("php-nextgen")
    inputSpec.set(view.path)
    outputDir.set(phpStaging.get().asFile.path)
    ignoreFileOverride.set(file("php.openapi-generator-ignore").path)
    // The library's ObjectSerializer and model templates: no Guzzle, no
    // Configuration, and enumUnknownDefaultCase honoured on decode.
    templateDir.set(file("templates/php").path)
    invokerPackage.set("Lingara\\Apps")
    modelPackage.set("Generated")
    globalProperties.set(
        mapOf(
            "models" to "",
            "modelTests" to "false",
            "modelDocs" to "false",
            "supportingFiles" to "ModelInterface.php,ObjectSerializer.php",
        ),
    )
    configOptions.set(
        mapOf(
            "hideGenerationTimestamp" to "true",
            "enumUnknownDefaultCase" to "true",
        ),
    )
    typeMappings.set(mapOf("DateTime" to "string", "UUID" to "string"))
}

tasks.register<Sync>("generatePhp") {
    dependsOn(generatePhpModels)
    outputs.upToDateWhen { false }
    from(phpStaging.map { it.dir("src") }) {
        include("Generated/**", "ObjectSerializer.php")
    }
    into(phpOut)
    preserve {
        include("**")
        exclude("Generated/**", "ObjectSerializer.php")
    }
}

tasks.test {
    systemProperty("lingara.view", view.path)
}
