package com.getlingara.apps;

import java.nio.charset.StandardCharsets;
import java.util.regex.Pattern;

/**
 * The string rules every card and manifest check shares (ADR 30.9.26am D5), each held to the
 * relay's definition rather than Java's nearest one:
 *
 * <ul>
 *   <li>a character is a Unicode scalar value ({@code codePointCount}), never a UTF-16 unit;
 *   <li>"empty" is empty after trimming Unicode {@code White_Space}, which {@code String.trim()}
 *       (only up to U+0020) and {@code Character.isWhitespace} (no U+00A0, but U+001C–U+001F) are
 *       not;
 *   <li>a pattern matches the whole string ({@code matches()}).
 * </ul>
 */
final class Text {
  private static final Pattern LANG = Pattern.compile("[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}");
  private static final Pattern ACTION = Pattern.compile("[A-Za-z0-9_.:-]{1,64}");
  private static final Pattern DIGITS = Pattern.compile("[0-9]+");
  private static final Pattern HEX = Pattern.compile("0[xX][0-9A-Fa-f]*");

  /** WHATWG's forbidden domain code points that can survive the control-character rule. */
  private static final String FORBIDDEN_HOST = " #%/:<>?@[\\]^|";

  private Text() {}

  /** The number of Unicode scalar values in {@code s}. */
  static int scalars(String s) {
    return s.codePointCount(0, s.length());
  }

  /**
   * Unicode's {@code White_Space} property, every code point of it: what Rust's {@code trim} trims.
   */
  private static final String WHITE_SPACE =
      "\t\n\u000B\f\r \u0085\u00A0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006"
          + "\u2007\u2008\u2009\u200A\u2028\u2029\u202F\u205F\u3000";

  static boolean isWhiteSpace(int c) {
    return c <= 0xFFFF && WHITE_SPACE.indexOf(c) >= 0;
  }

  /** {@code s} without leading or trailing Unicode {@code White_Space}. */
  static String trim(String s) {
    int start = 0;
    int end = s.length();
    while (start < end && isWhiteSpace(s.codePointAt(start))) {
      start += Character.charCount(s.codePointAt(start));
    }
    while (end > start && isWhiteSpace(s.codePointBefore(end))) {
      end -= Character.charCount(s.codePointBefore(end));
    }
    return s.substring(start, end);
  }

  /** C0 and C1 controls, and the bidi overrides and isolates. */
  static boolean isControlOrBidi(int c) {
    return c <= 0x1F
        || (c >= 0x7F && c <= 0x9F)
        || (c >= 0x202A && c <= 0x202E)
        || (c >= 0x2066 && c <= 0x2069);
  }

  /** Whether {@code s} holds a control or bidi character, {@code \n} excepted when allowed. */
  static boolean hasControl(String s, boolean newline) {
    return s.codePoints().anyMatch(c -> isControlOrBidi(c) && !(newline && c == '\n'));
  }

  /** Whether {@code s} holds a zero-width character: U+200B–U+200F or U+FEFF. */
  static boolean hasZeroWidth(String s) {
    return s.codePoints().anyMatch(c -> (c >= 0x200B && c <= 0x200F) || c == 0xFEFF);
  }

  static boolean isLangTag(String s) {
    return LANG.matcher(s).matches();
  }

  static boolean isActionId(String s) {
    return ACTION.matcher(s).matches();
  }

  static int utf8Length(String s) {
    return s.getBytes(StandardCharsets.UTF_8).length;
  }

  /**
   * {@code https}, no user information, a named host (never an IP literal), at most 2 048 bytes:
   * the relay's {@code link} rule. The host is read as a WHATWG URL parser reads it, so a host that
   * ends in a number ({@code 127.0.0.1}, {@code 0x7f.1}, {@code 2130706433}) is an IP address.
   */
  static boolean isSafeLink(String raw) {
    if (utf8Length(raw) > Limits.MAX_URL_BYTES) {
      return false;
    }
    String s = raw.replaceAll("^[\\x00-\\x20]+|[\\x00-\\x20]+$", "").replaceAll("[\\t\\n\\r]", "");
    int colon = s.indexOf(':');
    if (colon < 0 || !s.substring(0, colon).equalsIgnoreCase("https")) {
      return false;
    }
    String rest = s.substring(colon + 1).replaceFirst("^[/\\\\]*", "");
    int end = rest.length();
    for (char c : new char[] {'/', '\\', '?', '#'}) {
      int at = rest.indexOf(c);
      end = at >= 0 ? Math.min(end, at) : end;
    }
    String authority = rest.substring(0, end);
    int at = authority.lastIndexOf('@');
    if (at >= 0 && !authority.substring(0, at).replace(":", "").isEmpty()) {
      return false;
    }
    return isNamedHost(authority.substring(at + 1));
  }

  private static boolean isNamedHost(String hostPort) {
    if (hostPort.startsWith("[")) {
      return false;
    }
    int colon = hostPort.lastIndexOf(':');
    String host = colon >= 0 ? hostPort.substring(0, colon) : hostPort;
    String port = colon >= 0 ? hostPort.substring(colon + 1) : "";
    if (host.isEmpty() || !(port.isEmpty() || DIGITS.matcher(port).matches())) {
      return false;
    }
    if (host.chars().anyMatch(c -> FORBIDDEN_HOST.indexOf(c) >= 0)) {
      return false;
    }
    String trimmed = host.endsWith(".") ? host.substring(0, host.length() - 1) : host;
    String last = trimmed.substring(trimmed.lastIndexOf('.') + 1);
    return !(DIGITS.matcher(last).matches() || HEX.matcher(last).matches());
  }
}
