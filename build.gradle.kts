plugins {
    kotlin("jvm")
}

dependencies {
    implementation(project(":Shared"))
    implementation(project(":GeneralUtils"))
    implementation("com.github.ajalt.clikt:clikt:3.4.0")
    implementation("com.github.kittinunf.fuel:fuel:2.3.1")
    implementation("org.apache.commons:commons-lang3:3.11")
    implementation("org.apache.commons:commons-compress:1.21")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:${property("serialization.version")}")
}

tasks.compileKotlin {
    kotlinOptions.jvmTarget = "1.8"
}
