package com.getlingara.codegen;

import com.getlingara.codegen.AppsCodegen.Arm;
import com.getlingara.codegen.AppsCodegen.Field;
import com.getlingara.codegen.AppsCodegen.Operation;
import com.getlingara.codegen.AppsCodegen.Union;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

/**
 * AppsCodegen's Java output (ADR 30.9.26am D6): a union is a sealed interface carrying Jackson's
 * type information on its tag, an arm is a record that implements it, and the kinds and the
 * operations are enums. Null members are omitted on the wire, so a builder-made element never
 * writes {@code "lang": null}.
 */
final class JavaAppsSource implements AppsCodegen.Renderer {
  @Override
  public String union(String pkg, Union u) {
    List<String> types = new ArrayList<>();
    List<String> permits = new ArrayList<>();
    for (Arm a : u.arms()) {
      types.add(
          "  @JsonSubTypes.Type(value = " + a.name() + ".class, name = \"" + a.value() + "\")");
      permits.add(a.name());
    }
    return AppsCodegen.HEADER
        + "\npackage "
        + pkg
        + ";\n\n"
        + "import com.fasterxml.jackson.annotation.JsonSubTypes;\n"
        + "import com.fasterxml.jackson.annotation.JsonTypeInfo;\n\n"
        + "/** The {@code "
        + u.name()
        + "} union, tagged on {@code "
        + u.tag()
        + "}: one record per arm. */\n"
        + "@JsonTypeInfo(use = JsonTypeInfo.Id.NAME, include = JsonTypeInfo.As.PROPERTY,"
        + " property = \""
        + u.tag()
        + "\")\n"
        + "@JsonSubTypes({\n"
        + String.join(",\n", types)
        + "\n})\n"
        + "public sealed interface "
        + u.name()
        + " permits "
        + String.join(", ", permits)
        + " {}\n";
  }

  @Override
  public String arm(String pkg, Union u, Arm a) {
    List<String> components = new ArrayList<>();
    for (Field f : a.fields()) {
      components.add(
          "    @JsonProperty(\"" + f.json() + "\") " + javaType(f.type()) + " " + f.camel());
    }
    boolean list = a.fields().stream().anyMatch(f -> f.type().startsWith("list:"));
    return AppsCodegen.HEADER
        + "\npackage "
        + pkg
        + ";\n\n"
        + "import com.fasterxml.jackson.annotation.JsonInclude;\n"
        + (components.isEmpty() ? "" : "import com.fasterxml.jackson.annotation.JsonProperty;\n")
        + (list ? "import java.util.List;\n" : "")
        + "\n/** The {@code "
        + a.value()
        + "} arm of {@link "
        + u.name()
        + "}. */\n"
        + "@JsonInclude(JsonInclude.Include.NON_NULL)\n"
        + "public record "
        + a.name()
        + "("
        + (components.isEmpty() ? "" : "\n" + String.join(",\n", components))
        + ")\n    implements "
        + u.name()
        + " {}\n";
  }

  @Override
  public String kinds(String pkg, Union u) {
    List<String> constants = new ArrayList<>();
    for (Arm a : u.arms()) {
      constants.add("  " + constant(a.value()) + "(\"" + a.value() + "\")");
    }
    return AppsCodegen.HEADER
        + "\npackage "
        + pkg
        + ";\n\n"
        + "import com.fasterxml.jackson.annotation.JsonValue;\n\n"
        + "/** Every {@code "
        + u.name()
        + "} kind, in the view's order: the closed list a manifest's {@code context} takes. */\n"
        + "public enum "
        + u.name()
        + "Kind {\n"
        + String.join(",\n", constants)
        + ";\n\n"
        + enumBody(u.name() + "Kind")
        + "\n"
        + kindOf(u)
        + "}\n";
  }

  /** {@code of(slice)}: the kind of a decoded arm, by its type. */
  private static String kindOf(Union u) {
    StringBuilder b = new StringBuilder();
    b.append("  /**\n   * The kind of a decoded {@code ").append(u.name()).append("}.\n   *\n");
    b.append("   * @param slice the arm\n   * @return its kind\n   */\n");
    b.append("  public static ").append(u.name()).append("Kind of(").append(u.name());
    b.append(" slice) {\n");
    for (Arm a : u.arms()) {
      b.append("    if (slice instanceof ").append(a.name()).append(") {\n");
      b.append("      return ").append(constant(a.value())).append(";\n    }\n");
    }
    b.append("    throw new IllegalArgumentException(\"not an arm of ").append(u.name());
    b.append("\");\n  }\n");
    return b.toString();
  }

  @Override
  public String operations(String pkg, List<Operation> operations) {
    List<String> constants = new ArrayList<>();
    for (Operation o : operations) {
      constants.add("  /** Its request is {@link " + o.request() + "}. */");
      constants.add("  " + o.constant() + "(\"" + o.message() + "\")");
    }
    String reply = operations.isEmpty() ? "" : operations.get(0).replyMessage();
    return AppsCodegen.HEADER
        + "\npackage "
        + pkg
        + ";\n\n"
        + "import com.fasterxml.jackson.annotation.JsonValue;\n\n"
        + "/** The app operations: each request's {@code type}, a closed set. */\n"
        + "public enum AppOperation {\n"
        + joinConstants(constants)
        + ";\n\n"
        + "  /** The message every operation's reply is. */\n"
        + "  public static final String REPLY_MESSAGE = \""
        + reply
        + "\";\n\n"
        + enumBody("AppOperation")
        + "}\n";
  }

  /** The constants, a comma after each but the last (every other line is a doc comment). */
  private static String joinConstants(List<String> lines) {
    StringBuilder b = new StringBuilder();
    for (int i = 0; i < lines.size(); i++) {
      b.append(lines.get(i));
      if (i % 2 == 1 && i < lines.size() - 1) {
        b.append(',');
      }
      b.append('\n');
    }
    return b.toString().stripTrailing();
  }

  private static String enumBody(String name) {
    return "  private final String value;\n\n"
        + "  "
        + name
        + "(String value) {\n"
        + "    this.value = value;\n"
        + "  }\n\n"
        + "  /**\n"
        + "   * Its spelling on the wire.\n"
        + "   *\n"
        + "   * @return the value\n"
        + "   */\n"
        + "  @JsonValue\n"
        + "  public String getValue() {\n"
        + "    return value;\n"
        + "  }\n\n"
        + "  /**\n"
        + "   * The constant spelled {@code value} on the wire.\n"
        + "   *\n"
        + "   * @param value the wire spelling\n"
        + "   * @return the constant, or empty for a value this version does not know\n"
        + "   */\n"
        + "  public static java.util.Optional<"
        + name
        + "> fromValue(String value) {\n"
        + "    for ("
        + name
        + " c : values()) {\n"
        + "      if (c.value.equals(value)) {\n"
        + "        return java.util.Optional.of(c);\n"
        + "      }\n"
        + "    }\n"
        + "    return java.util.Optional.empty();\n"
        + "  }\n";
  }

  static String constant(String value) {
    return value.replaceAll("[^A-Za-z0-9]", "_").toUpperCase(Locale.ROOT);
  }

  static String javaType(String type) {
    if (type.startsWith("ref:")) {
      return type.substring(4);
    }
    if (type.startsWith("list:")) {
      return "List<" + type.substring(5) + ">";
    }
    return switch (type) {
      case "string" -> "String";
      case "bool" -> "Boolean";
      case "double" -> "Double";
      case "u8" -> "Integer";
      default -> "Long";
    };
  }
}
