# frozen_string_literal: true

require "test_helper"

class RackTest < Minitest::Test
  def setup
    @logger = KitTest::Recorder.new
    @renders = []
    @actions = []
    app = Lingara::Apps::App.new(secret: KitTest::SECRET, logger: @logger)
    app.render do |request|
      @renders << request
      Lingara::Apps.card.heading(request.slot).build
    end
    app.action("inc") do |request|
      @actions << request
      Lingara::Apps.card.text(request.card_etag).build
    end
    app.action("boom") { raise "boom" }
    @rack = Lingara::Apps::RackApp.new(app)
  end

  def call(body, **options)
    @rack.call(KitTest.signed_env(body, **options))
  end

  # 30.9.26am AC13: through RackApp with a Rack env, a tampered body is 401
  # and never reaches the render function; a valid render reaches it with
  # the decoded subject, slot and slices, an unknown slice kind skipped; an
  # action's card_etag reaches its function byte for byte.
  def test_the_rack_app_verifies_the_raw_body_before_dispatch
    body = JSON.generate(KitTest.request("app.render"))
    env = KitTest.signed_env(body)
    env["rack.input"] = StringIO.new(body.sub("home.side", "home.sidf"))
    assert_equal [401, {}, []], @rack.call(env)
    assert_empty @renders

    slices = [{kind: "languages", source_lang: "en", target_lang: "zh", level: 2, extra: "ignored"},
      {kind: "telepathy", vibe: "strong"}, {kind: "review_due", due: 3, learned: 40}]
    status, headers, chunks = call(JSON.generate(KitTest.request("app.render", context: slices, unknown_field: 1)))
    assert_equal [200, "application/json"], [status, headers["content-type"]]
    assert_equal({"card" => {"elements" => [{"type" => "heading", "text" => "home.side", "level" => 1}]}}, JSON.parse(chunks.join))
    request = @renders.fetch(0)
    assert_instance_of Lingara::Apps::AppRenderRequest, request
    assert_equal ["lgr_sub_unit", "home.side"], [request.subject, request.slot]
    assert_equal [Lingara::Apps::ContextSliceLanguages.new(source_lang: "en", target_lang: "zh", level: 2),
      Lingara::Apps::ContextSliceReviewDue.new(due: 3, learned: 40)], request.context
    assert(@logger.lines.any? { |line| line.include?("telepathy") })

    etag = "c1_AbC-/+=._~ é"
    status, _, chunks = call(JSON.generate(KitTest.request("app.action", action_id: "inc", card_etag: etag)))
    assert_equal 200, status
    assert_equal etag, @actions.fetch(0).card_etag
    assert_equal etag, JSON.parse(chunks.join).dig("card", "elements", 0, "text")
  end

  def test_the_rack_app_answers_the_contract_errors
    assert_equal [405, {}, []], call("{}", method: "GET")
    assert_equal 413, call(" " * 65_537).first
    bad = [400, {"content-type" => "application/json"}, ['{"error":"bad_request"}']]
    assert_equal bad, call("not json")
    assert_equal bad, call(JSON.generate(KitTest.request("app.unknown")))
    assert_equal bad, call(JSON.generate(KitTest.request("app.action", action_id: "nope", card_etag: "c1_x")))
    failed = [500, {"content-type" => "application/json"}, ['{"error":"handler_failed"}']]
    assert_equal failed, call(JSON.generate(KitTest.request("app.action", action_id: "boom", card_etag: "c1_x")))
    assert(@logger.lines.any? { |line| line.include?("app.action") && line.include?("boom") })
    assert_raises(ArgumentError) { Lingara::Apps::App.new(secret: "lgr_whsec_not base64") }
  end
end
