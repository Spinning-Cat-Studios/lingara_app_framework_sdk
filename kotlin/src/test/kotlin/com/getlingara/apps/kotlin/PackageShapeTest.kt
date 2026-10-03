package com.getlingara.apps.kotlin

import org.w3c.dom.Element
import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class PackageShapeTest {
    /**
     * 30.9.26am AC26: the generated POM's only dependency is lingara-kotlin (kotlin-stdlib comes
     * through it); no io.ktor artifact appears.
     */
    @Test
    fun onlyTheLibraryIsRequired() {
        val factory = DocumentBuilderFactory.newInstance()
        factory.setFeature("http://apache.org/xml/features/disallow-doctype-decl", true)
        val pom = File(System.getProperty("lingara.pom"))
        val deps = factory.newDocumentBuilder().parse(pom).getElementsByTagName("dependency")
        val coordinates =
            (0 until deps.length).map { i ->
                val d = deps.item(i) as Element
                "${text(d, "groupId")}:${text(d, "artifactId")}:${text(d, "scope")}"
            }
        assertEquals(listOf("com.getlingara:lingara-kotlin:compile"), coordinates)
        assertTrue("io.ktor" !in pom.readText())
    }

    private fun text(
        parent: Element,
        tag: String,
    ): String =
        parent
            .getElementsByTagName(tag)
            .item(0)
            ?.textContent
            ?.trim() ?: ""
}
