// The root project of the shared Gradle build (ADR 30.9.26am D8). It builds
// nothing itself. Loading the convention plugins here, unapplied, puts them
// on the root classloader once, so a build service they register (Spotless's)
// is one service for every project rather than one per sibling.

plugins {
    id("lingara.java-conventions") apply false
    id("lingara.publishing-conventions") apply false
    id("lingara.kotlin-conventions") apply false
}
