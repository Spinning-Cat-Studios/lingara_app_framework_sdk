# frozen_string_literal: true

require "test_helper"

class VectorsTest < Minitest::Test
  # The reason validate_reply gives, as the vectors spell it.
  def card_answer(reply)
    Lingara::Apps.validate_reply(reply)
    {"ok" => true}
  rescue Lingara::Apps::CardLimitError => e
    {"refused" => e.reason.to_s}
  end

  def manifest_answer(manifest)
    Lingara::Apps.validate_manifest(manifest)
    {"ok" => true}
  rescue Lingara::Apps::ManifestError => e
    {"refused" => e.rule.to_s}
  end

  # 30.9.26am AC6: every card-limits.json vector's expect equals
  # validate_reply's answer, every manifest.json vector's equals
  # validate_manifest's, and truncate of an 81-scalar astral heading at 80 is
  # 79 scalars plus `…`.
  def test_every_card_and_manifest_vector_gives_its_expected_answer
    cards = KitTest.vectors("card-limits.json")
    refute_empty cards
    cards.each { |v| assert_equal v["expect"], card_answer(v["reply"]), v["name"] }

    manifests = KitTest.vectors("manifest.json")
    refute_empty manifests
    manifests.each { |v| assert_equal v["expect"], manifest_answer(v["manifest"]), v["name"] }

    heading = "𝄞" * 81
    cut = Lingara::Apps.truncate(heading, 80)
    assert_equal 80, cut.length
    assert_equal "#{"𝄞" * 79}…", cut
    assert_equal "𝄞" * 80, Lingara::Apps.truncate("𝄞" * 80, 80)
  end

  # On success validate_reply returns the bytes it measured: compact JSON,
  # raw UTF-8, `/` unescaped, an absent tutor_note omitted.
  def test_validate_reply_returns_the_bytes_it_measured
    reply = {"card" => {"elements" => [{"type" => "link", "label" => "漢字/kanji", "url" => "https://apps.example.com/a"}]}}
    bytes = Lingara::Apps.validate_reply(reply)
    assert_equal '{"card":{"elements":[{"type":"link","label":"漢字/kanji","url":"https://apps.example.com/a"}]}}', bytes
  end

  def test_the_card_builder_refuses_with_the_reason
    items = (1..21).map { |i| Lingara::Apps.item.text(i.to_s) }
    error = assert_raises(Lingara::Apps::CardLimitError) { Lingara::Apps.card.list(items).build }
    assert_equal :list_items, error.reason
    error = assert_raises(Lingara::Apps::CardLimitError) { Lingara::Apps.card.heading("　").build }
    assert_equal :empty_element, error.reason
    error = assert_raises(Lingara::Apps::CardLimitError) { Lingara::Apps.card.link("Why", "https://127.1/").build }
    assert_equal :link, error.reason
    error = assert_raises(Lingara::Apps::CardLimitError) { Lingara::Apps.card.text("x", lang: "en\nfr").build }
    assert_equal :control_chars, error.reason
    assert_raises(Lingara::Apps::CardLimitError) { Lingara::Apps.card.build }
  end

  def test_the_builders_write_the_wire_form
    card = Lingara::Apps.card.heading("Today", 2).text("a\nb").term(word: "雨", reading: "yǔ").divider
      .progress(0.5, "half").button("Next", "next", style: "primary").link("Why", "https://apps.example.com/why").build
    wire = Lingara::Apps.reply(card).tutor_note(" a note\n").to_wire
    assert_equal({"type" => "term", "word" => "雨", "reading" => "yǔ"}, wire["card"]["elements"][2])
    assert_equal({"type" => "divider"}, wire["card"]["elements"][3])
    assert_equal " a note\n", wire["tutor_note"]
    assert_equal 7, card.elements.size
    assert(card.elements.all? { |e| e.is_a?(Lingara::Apps::CardElement) })
    error = assert_raises(Lingara::Apps::CardLimitError) { Lingara::Apps.reply(card).tutor_note("a" * 281) }
    assert_equal :tutor_note_length, error.reason
  end

  # 4.10.26e AC5: a real scope (`lesson_plans:read`) builds; the same one twice
  # is refused as `duplicate`.
  def test_the_manifest_builder_writes_the_upload
    manifest = Lingara::Apps.manifest.default_locale("en").name("Daily five").description({"en" => "Five words."})
      .render_url("https://apps.example.com/lingara").slots("home.side").context("languages").scopes("lesson_plans:read")
      .tutor_note.build
    assert_equal({"en" => "Daily five"}, manifest["name"])
    assert_equal ["lesson_plans:read"], manifest["scopes"]
    assert_equal [true, false], [manifest["tutor_note"], manifest["listed"]]
    error = assert_raises(Lingara::Apps::ManifestError) { Lingara::Apps.manifest.default_locale("en").name("x").build }
    assert_equal :description, error.rule
    error = assert_raises(Lingara::Apps::ManifestError) do
      Lingara::Apps.manifest.default_locale("en").name("n").description("d").render_url("https://a.example")
        .slots("home.side").scopes("lesson_plans:read", "lesson_plans:read").build
    end
    assert_equal :duplicate, error.rule
    json = JSON.parse(Lingara::Apps.manifest.default_locale("en").name("n").description("d").render_url("https://a.example")
      .slots("home.side").to_json)
    assert_equal 1, json["manifest_version"]
  end
end
