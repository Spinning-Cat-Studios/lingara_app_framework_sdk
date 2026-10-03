package com.getlingara.codegen;

import com.getlingara.codegen.AppsCodegen.Arm;
import com.getlingara.codegen.AppsCodegen.Field;
import com.getlingara.codegen.AppsCodegen.Operation;
import com.getlingara.codegen.AppsCodegen.Union;
import java.util.ArrayList;
import java.util.List;

/**
 * AppsCodegen's Kotlin output (ADR 30.9.26am D6): a union is a {@code Serializable} sealed
 * interface whose class discriminator is its tag, an arm is a {@code Serializable} data class (a
 * data object when it has no field) that implements it, and the kinds and the operations are enum
 * classes. An optional member defaults to null, which a {@code Json} without {@code encodeDefaults}
 * omits.
 */
final class KotlinAppsSource implements AppsCodegen.Renderer {
  @Override
  public String union(String pkg, Union u) {
    return AppsCodegen.HEADER
        + "@file:OptIn(ExperimentalSerializationApi::class)\n\n"
        + "package "
        + pkg
        + "\n\n"
        + "import kotlinx.serialization.ExperimentalSerializationApi\n"
        + "import kotlinx.serialization.Serializable\n"
        + "import kotlinx.serialization.json.JsonClassDiscriminator\n\n"
        + "/** The `"
        + u.name()
        + "` union, tagged on `"
        + u.tag()
        + "`: one class per arm. */\n"
        + "@Serializable\n"
        + "@JsonClassDiscriminator(\""
        + u.tag()
        + "\")\n"
        + "public sealed interface "
        + u.name()
        + "\n";
  }

  @Override
  public String arm(String pkg, Union u, Arm a) {
    String head =
        AppsCodegen.HEADER
            + "package "
            + pkg
            + "\n\n"
            + "import kotlinx.serialization.SerialName\n"
            + "import kotlinx.serialization.Serializable\n\n"
            + "/** The `"
            + a.value()
            + "` arm of ["
            + u.name()
            + "]. */\n"
            + "@Serializable\n"
            + "@SerialName(\""
            + a.value()
            + "\")\n";
    if (a.fields().isEmpty()) {
      return head + "public data object " + a.name() + " : " + u.name() + "\n";
    }
    List<String> params = new ArrayList<>();
    for (Field f : a.fields()) {
      params.add(
          "    @SerialName(\""
              + f.json()
              + "\") public val "
              + f.camel()
              + ": "
              + kotlinType(f.type())
              + (f.optional() ? "? = null" : "")
              + ",");
    }
    return head
        + "public data class "
        + a.name()
        + "(\n"
        + String.join("\n", params)
        + "\n) : "
        + u.name()
        + "\n";
  }

  @Override
  public String kinds(String pkg, Union u) {
    List<String> constants = new ArrayList<>();
    for (Arm a : u.arms()) {
      constants.add(
          "    @SerialName(\""
              + a.value()
              + "\")\n    "
              + JavaAppsSource.constant(a.value())
              + "(\""
              + a.value()
              + "\")");
    }
    return AppsCodegen.HEADER
        + "package "
        + pkg
        + "\n\n"
        + "import kotlinx.serialization.SerialName\n"
        + "import kotlinx.serialization.Serializable\n\n"
        + "/** Every `"
        + u.name()
        + "` kind, in the view's order: the closed list a manifest's `context` takes. */\n"
        + "@Serializable\n"
        + "public enum class "
        + u.name()
        + "Kind(\n    public val value: String,\n) {\n"
        + String.join(",\n", constants)
        + ",\n    ;\n\n"
        + companion(u.name() + "Kind", kindOf(u))
        + "}\n";
  }

  @Override
  public String operations(String pkg, List<Operation> operations) {
    List<String> constants = new ArrayList<>();
    for (Operation o : operations) {
      constants.add(
          "    /** Its request is ["
              + o.request()
              + "]. */\n    "
              + o.constant()
              + "(\""
              + o.message()
              + "\")");
    }
    String reply = operations.isEmpty() ? "" : operations.get(0).replyMessage();
    return AppsCodegen.HEADER
        + "package "
        + pkg
        + "\n\n"
        + "/** The app operations: each request's `type`, a closed set. */\n"
        + "public enum class AppOperation(\n    public val value: String,\n) {\n"
        + String.join(",\n", constants)
        + ",\n    ;\n\n"
        + companion(
            "AppOperation",
            "        /** The message every operation's reply is. */\n"
                + "        public const val REPLY_MESSAGE: String = \""
                + reply
                + "\"\n\n")
        + "}\n";
  }

  /** {@code of(slice)}: the kind of a decoded arm, by its type, exhaustively. */
  private static String kindOf(Union u) {
    StringBuilder b = new StringBuilder();
    b.append("        /** The kind of a decoded [").append(u.name()).append("]. */\n");
    b.append("        public fun of(slice: ").append(u.name()).append("): ");
    b.append(u.name()).append("Kind =\n            when (slice) {\n");
    for (Arm a : u.arms()) {
      b.append("                is ").append(a.name()).append(" -> ");
      b.append(JavaAppsSource.constant(a.value())).append("\n");
    }
    b.append("            }\n\n");
    return b.toString();
  }

  private static String companion(String name, String extra) {
    return "    public companion object {\n"
        + extra
        + "        /** The constant spelled [value] on the wire, or null for one this version"
        + " does not know. */\n"
        + "        public fun fromValue(value: String): "
        + name
        + "? = entries.firstOrNull { it.value == value }\n"
        + "    }\n";
  }

  static String kotlinType(String type) {
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
      case "u8" -> "Int";
      default -> "Long";
    };
  }
}
