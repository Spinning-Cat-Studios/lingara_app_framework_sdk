package com.getlingara.apps.kotlin

/**
 * The string rules every card and manifest check shares (ADR 30.9.26am D5), each held to the
 * relay's definition: a character is a Unicode scalar value (`codePointCount`), "empty" is empty
 * after trimming Unicode `White_Space` (never `String.trim()`'s idea of it), and a pattern matches
 * the whole string (`matches`).
 */
internal object Text {
    private val LANG = Regex("[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}")
    private val ACTION = Regex("[A-Za-z0-9_.:-]{1,64}")
    private val DIGITS = Regex("[0-9]+")
    private val HEX = Regex("0[xX][0-9A-Fa-f]*")
    private val EDGES = Regex("^[\\x00-\\x20]+|[\\x00-\\x20]+$")
    private val LEADING_SLASHES = Regex("^[/\\\\]*")

    /** Every code point of Unicode's `White_Space`: what Rust's `trim` trims. */
    private const val WHITE_SPACE =
        "\t\n\u000B\u000C\r \u0085         " +
            "        　"

    /** WHATWG's forbidden domain code points that can survive the control-character rule. */
    private const val FORBIDDEN_HOST = " #%/:<>?@[\\]^|"

    fun scalars(s: String): Int = s.codePointCount(0, s.length)

    fun isWhiteSpace(c: Int): Boolean = c <= 0xFFFF && WHITE_SPACE.indexOf(c.toChar()) >= 0

    /** [s] without leading or trailing Unicode `White_Space`. */
    fun trim(s: String): String {
        var start = 0
        var end = s.length
        while (start < end && isWhiteSpace(s.codePointAt(start))) start += Character.charCount(s.codePointAt(start))
        while (end > start && isWhiteSpace(s.codePointBefore(end))) end -= Character.charCount(s.codePointBefore(end))
        return s.substring(start, end)
    }

    /** C0 and C1 controls, and the bidi overrides and isolates. */
    fun isControlOrBidi(c: Int): Boolean = c <= 0x1F || c in 0x7F..0x9F || c in 0x202A..0x202E || c in 0x2066..0x2069

    fun hasControl(
        s: String,
        newline: Boolean,
    ): Boolean = s.codePoints().anyMatch { isControlOrBidi(it) && !(newline && it == '\n'.code) }

    fun hasZeroWidth(s: String): Boolean = s.codePoints().anyMatch { it in 0x200B..0x200F || it == 0xFEFF }

    fun isLangTag(s: String): Boolean = LANG.matches(s)

    fun isActionId(s: String): Boolean = ACTION.matches(s)

    fun utf8Length(s: String): Int = s.toByteArray(Charsets.UTF_8).size

    /**
     * `https`, no user information, a named host (never an IP literal), at most 2 048 bytes. The
     * host is read as a WHATWG URL parser reads it, so a host that ends in a number is an IP address.
     */
    fun isSafeLink(raw: String): Boolean {
        if (utf8Length(raw) > Limits.MAX_URL_BYTES) return false
        val s = raw.replace(EDGES, "").filterNot { it == '\t' || it == '\n' || it == '\r' }
        val colon = s.indexOf(':')
        if (colon < 0 || !s.substring(0, colon).equals("https", ignoreCase = true)) return false
        val rest = s.substring(colon + 1).replaceFirst(LEADING_SLASHES, "")
        val end = rest.indexOfFirst { it in "/\\?#" }.let { if (it < 0) rest.length else it }
        val authority = rest.substring(0, end)
        val at = authority.lastIndexOf('@')
        if (at >= 0 && authority.substring(0, at).replace(":", "").isNotEmpty()) return false
        return isNamedHost(authority.substring(at + 1))
    }

    private fun isNamedHost(hostPort: String): Boolean {
        if (hostPort.startsWith("[")) return false
        val colon = hostPort.lastIndexOf(':')
        val host = if (colon >= 0) hostPort.substring(0, colon) else hostPort
        val port = if (colon >= 0) hostPort.substring(colon + 1) else ""
        if (host.isEmpty() || !(port.isEmpty() || DIGITS.matches(port))) return false
        if (host.any { it in FORBIDDEN_HOST }) return false
        val last = host.removeSuffix(".").substringAfterLast('.')
        return !(DIGITS.matches(last) || HEX.matches(last))
    }
}
