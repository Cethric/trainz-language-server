plugins {
    id("java")
    alias(libs.plugins.kotlin)
    alias(libs.plugins.intellijPlatform)
}

group = "io.github.cethric"
version = "1.0.0-SNAPSHOT"

// Set the JVM language level used to build the project.
// Target JVM 17 bytecode for compatibility with IntelliJ IDEA 2024.3 (JBR 21).
kotlin {
    jvmToolchain(23)
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

repositories {
    mavenCentral()
    intellijPlatform {
        defaultRepositories()
    }
}

// Read more: https://plugins.jetbrains.com/docs/intellij/tools-intellij-platform-gradle-plugin.html
dependencies {
    intellijPlatform {
        intellijIdeaCommunity(providers.gradleProperty("platformVersion"))
        testFramework(org.jetbrains.intellij.platform.gradle.TestFrameworkType.Platform)

        // Add LSP4IJ plugin dependency for LSP integration
        plugins(providers.gradleProperty("platformPlugins").map { it.split(',').map(String::trim).filter(String::isNotEmpty) })
    }
}

intellijPlatform {
    pluginConfiguration {
        ideaVersion {
            sinceBuild = providers.gradleProperty("pluginSinceBuild")
        }

        changeNotes = """
            Initial version
        """.trimIndent()
    }
}

tasks {
    wrapper {
        gradleVersion = providers.gradleProperty("gradleVersion").get()
    }

    // Copy shared icons from the root icons/ directory into the plugin resources
    processResources {
        from(rootProject.file("../../icons")) {
            include("gs.svg", "acs.svg")
            into("icons")
        }
        from(rootProject.file("../../icons")) {
            include("plugin.svg")
            rename("plugin.svg", "pluginIcon.svg")
            into("META-INF")
        }
    }
}
