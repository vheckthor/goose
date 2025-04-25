plugins {
    kotlin("jvm") version "1.9.0"
    application
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("net.java.dev.jna:jna:5.13.0")
    implementation(kotlin("stdlib"))
}

application {
    mainClass.set("GooseExampleKt")
}

tasks.withType<JavaExec> {
    // Set the library path to include the directory where the goose_ffi library is located
    systemProperty("jna.library.path", "../../target/debug")
}