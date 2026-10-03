package com.getlingara.codegen;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class AppsCodegenTest {
  private static final Map<String, List<String>> ARMS =
      Map.of(
          "CardElement",
          List.of(
              "CardElementHeading",
              "CardElementText",
              "CardElementTerm",
              "CardElementList",
              "CardElementProgress",
              "CardElementDivider",
              "CardElementButton",
              "CardElementLink"),
          "ListItem",
          List.of("ListItemText", "ListItemTerm"),
          "ContextSlice",
          List.of(
              "ContextSliceLanguages",
              "ContextSlicePlanSummary",
              "ContextSliceReviewDue",
              "ContextSliceTutorTopic"));

  @TempDir Path sources;

  private static JsonNode view() throws Exception {
    return new ObjectMapper().readTree(Path.of(System.getProperty("lingara.view")).toFile());
  }

  private String read(AppsCodegen.Lang lang, String type) throws Exception {
    Path dir = sources.resolve(lang.model().replace('.', '/'));
    return Files.readString(dir.resolve(type + lang.extension()));
  }

  /**
   * 30.9.26am AC19: over the committed view, AppsCodegen writes, for Java and for Kotlin, the three
   * sealed unions with S1's lifted arm names, each arm implementing its union, the four
   * ContextSlice kinds as a constant list, and the app.render / app.action operations with the
   * app.card reply; and it refuses a staged file named after a union or an arm.
   */
  @Test
  void writesTheThreeUnionsAndTheOperations() throws Exception {
    for (AppsCodegen.Lang lang : List.of(AppsCodegen.Lang.JAVA, AppsCodegen.Lang.KOTLIN)) {
      new AppsCodegen(view(), sources, lang).write();
      boolean java = lang == AppsCodegen.Lang.JAVA;
      for (Map.Entry<String, List<String>> union : ARMS.entrySet()) {
        String sealed = read(lang, union.getKey());
        assertTrue(sealed.contains("public sealed interface " + union.getKey()), sealed);
        if (java) {
          assertTrue(sealed.contains("permits " + String.join(", ", union.getValue())), sealed);
        }
        for (String arm : union.getValue()) {
          String implementsIt = java ? "implements " + union.getKey() : ") : " + union.getKey();
          String source = read(lang, arm);
          assertTrue(
              source.contains(implementsIt) || source.contains("object " + arm + " : "), source);
        }
      }
      String kinds = read(lang, "ContextSliceKind");
      for (String kind : List.of("languages", "plan_summary", "review_due", "tutor_topic")) {
        assertTrue(kinds.contains("(\"" + kind + "\")"), kinds);
      }
      String ops = read(lang, "AppOperation");
      assertTrue(ops.contains("APP_RENDER(\"app.render\")"), ops);
      assertTrue(ops.contains("APP_ACTION(\"app.action\")"), ops);
      assertTrue(ops.contains("REPLY_MESSAGE") && ops.contains("\"app.card\""), ops);
      refusesAGeneratorCopy(lang);
    }
  }

  private static void refusesAGeneratorCopy(AppsCodegen.Lang lang) throws Exception {
    for (String copy : List.of("CardElement", "ListItemTerm", "ContextSliceLanguages")) {
      Path fresh = Files.createTempDirectory("staged");
      Path models = fresh.resolve(lang.model().replace('.', '/'));
      Files.createDirectories(models);
      Files.writeString(models.resolve(copy + lang.extension()), "");
      IllegalStateException e =
          assertThrows(
              IllegalStateException.class, () -> new AppsCodegen(view(), fresh, lang).write());
      assertTrue(e.getMessage().contains(copy + lang.extension()), e.getMessage());
      assertTrue(e.getMessage().contains(lang.ignoreFile() + ".openapi-generator-ignore"));
    }
  }

  @Test
  void typesTheArmsFieldsFromTheView() throws Exception {
    AppsCodegen codegen = new AppsCodegen(view(), sources, AppsCodegen.Lang.JAVA);
    List<AppsCodegen.Union> unions = codegen.unions();
    assertEquals(List.of("ContextSlice", "CardElement", "ListItem"), names(unions));
    AppsCodegen.Arm button = unions.get(1).arms().get(6);
    assertEquals("CardElementButton", button.name());
    assertEquals("ref:ButtonStyle", button.fields().get(2).type());
    assertTrue(button.fields().get(2).optional());
    AppsCodegen.Arm summary = unions.get(0).arms().get(1);
    assertEquals("planId", summary.fields().get(0).camel());
    assertEquals("u32", summary.fields().get(3).type());
  }

  private static List<String> names(List<AppsCodegen.Union> unions) {
    return unions.stream().map(AppsCodegen.Union::name).toList();
  }
}
