package com.getlingara.apps;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;

class VectorsTest {
  private static JsonNode vectors(String property) throws Exception {
    return new ObjectMapper().readTree(Path.of(System.getProperty(property)).toFile());
  }

  private static String cardAnswer(JsonNode reply) {
    try {
      byte[] sent = Limits.validateReply(reply);
      assertArrayEquals(Json.MAPPER.writeValueAsBytes(reply), sent);
      return "ok";
    } catch (CardLimitException e) {
      return e.reason().wire();
    } catch (Exception e) {
      throw new AssertionError(e);
    }
  }

  private static String manifestAnswer(JsonNode manifest) {
    try {
      Manifest.validateManifest(manifest);
      return "ok";
    } catch (ManifestException e) {
      return e.rule();
    }
  }

  private static String expected(JsonNode expect) {
    return expect.path("ok").asBoolean(false) ? "ok" : expect.path("refused").asText();
  }

  /**
   * 30.9.26am AC4: every card-limits.json vector's expect equals validateReply's answer, every
   * manifest.json vector's equals validateManifest's, and truncate of an 81-scalar astral heading
   * at 80 is 79 scalars plus an ellipsis.
   */
  @Test
  void everyCardAndManifestVectorGivesItsExpectedAnswer() throws Exception {
    for (JsonNode v : vectors("lingara.cardVectors").path("vectors")) {
      assertEquals(
          expected(v.path("expect")), cardAnswer(v.path("reply")), v.path("name").asText());
    }
    for (JsonNode v : vectors("lingara.manifestVectors").path("vectors")) {
      assertEquals(
          expected(v.path("expect")), manifestAnswer(v.path("manifest")), v.path("name").asText());
    }
    String astral = "𝄞".repeat(81);
    String cut = Limits.truncate(astral, 80);
    assertEquals(80, Text.scalars(cut));
    assertEquals("𝄞".repeat(79) + "…", cut);
    assertEquals("𝄞".repeat(80), Limits.truncate("𝄞".repeat(80), 80));
  }

  @Test
  void theBuildersRefuseWhatTheRelayWouldClamp() {
    List<Item> items = new java.util.ArrayList<>();
    for (int i = 1; i <= 21; i++) {
      items.add(Item.text(Integer.toString(i)));
    }
    CardLimitException overflow =
        assertThrows(CardLimitException.class, () -> Card.card().list(items).build());
    assertEquals(Limits.Reason.LIST_ITEMS, overflow.reason());
    Card.Builder huge = Card.card();
    for (int i = 0; i < 24; i++) {
      huge.text("漢".repeat(600));
    }
    Card card = huge.build();
    CardLimitException tooLarge =
        assertThrows(
            CardLimitException.class,
            () -> Limits.validateReply(Json.MAPPER.createObjectNode().set("card", card.toTree())));
    assertEquals(Limits.Reason.REPLY_TOO_LARGE, tooLarge.reason());
    CardLimitException note =
        assertThrows(
            CardLimitException.class,
            () -> Reply.reply(Card.card().divider().build()).tutorNote("a".repeat(281)));
    assertEquals(Limits.Reason.TUTOR_NOTE_LENGTH, note.reason());
    assertEquals(
        Limits.Reason.LINK,
        assertThrows(
                CardLimitException.class,
                () -> Card.card().link("Why", "https://127.0.0.1/").build())
            .reason());
  }

  @Test
  void aBuilderMadeCardOmitsItsAbsentMembers() {
    Card card =
        Card.card()
            .heading("Today", 1)
            .term("雨", "yǔ", "rain", "zh")
            .list(Item.text("one"), Item.term("二"))
            .button("Next", "next")
            .build();
    assertEquals(
        "{\"elements\":[{\"type\":\"heading\",\"text\":\"Today\",\"level\":1},"
            + "{\"type\":\"term\",\"word\":\"雨\",\"reading\":\"yǔ\",\"gloss\":\"rain\",\"lang\":\"zh\"},"
            + "{\"type\":\"list\",\"items\":[{\"type\":\"text\",\"text\":\"one\"},"
            + "{\"type\":\"term\",\"word\":\"二\"}]},"
            + "{\"type\":\"button\",\"label\":\"Next\",\"action\":\"next\"}]}",
        card.toTree().toString());
  }

  @Test
  void theManifestBuilderWritesTheWireForm() {
    String json =
        Manifest.manifest()
            .defaultLocale("en")
            .name("Daily five")
            .description("Five words to review.")
            .renderUrl("https://apps.example.com/lingara/render")
            .slots(com.getlingara.apps.model.AppSlotName.HOME_SIDE)
            .context(com.getlingara.apps.model.ContextSliceKind.LANGUAGES)
            .build()
            .toJson();
    assertEquals(
        "{\"manifest_version\":1,\"default_locale\":\"en\",\"name\":{\"en\":\"Daily five\"},"
            + "\"description\":{\"en\":\"Five words to review.\"},"
            + "\"render_url\":\"https://apps.example.com/lingara/render\","
            + "\"slots\":[\"home.side\"],\"context\":[\"languages\"],\"scopes\":[],"
            + "\"tutor_note\":false,\"listed\":false}",
        json);
    ManifestException e =
        assertThrows(
            ManifestException.class,
            () -> Manifest.manifest().defaultLocale("en").name("x").description("y").build());
    assertEquals("render_url", e.rule());
  }
}
