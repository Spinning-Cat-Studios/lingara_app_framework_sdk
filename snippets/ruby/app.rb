# frozen_string_literal: true

# The Ruby kit examples the documentation site shows. Each marked region is
# vendored at a released tag; make test-ruby parses this file under -w and
# loads it against the kit.

require "lingara/apps"

module LingaraAppsSnippets
  STREAKS = Hash.new(0)

  # lingara:begin card
  def self.today_card(streak)
    Lingara::Apps.card
      .heading("Today's five", 1)
      .term(word: "雨", reading: "yǔ", gloss: "rain", lang: "zh")
      .list([Lingara::Apps.item.text("Review 3 words"), Lingara::Apps.item.term(word: "二", reading: "èr")])
      .button("Done (#{streak})", "done")
      .build # Raises Lingara::Apps::CardLimitError naming the rule Lingara would clamp.
  end
  # lingara:end

  def self.plan_line(request)
    # lingara:begin context
    # The app's own client reads its owner's account, never the learner's. The
    # slices are all a render knows about the learner.
    #
    # A slice is present only when the learner agreed to share it.
    request.context.filter_map do |slice|
      case slice
      in Lingara::Apps::ContextSliceLanguages(source_lang:, target_lang:)
        "#{source_lang} → #{target_lang}."
      in Lingara::Apps::ContextSlicePlanSummary(sets_completed:, set_count:)
        "#{sets_completed}/#{set_count} sets."
      else nil
      end
    end.join(" ")
    # lingara:end
  end

  def self.with_note(request)
    # lingara:begin tutorNote
    note = plan_line(request)
    # Plain text, at most 280 characters; the tutor reads it, the learner does not.
    note.empty? ? today_card(0) : Lingara::Apps.reply(today_card(0)).tutor_note(note)
    # lingara:end
  end

  def self.app
    # lingara:begin handler
    # lgr_whsec_…, from the app's settings; pass an Array of two during a rotation.
    app = Lingara::Apps::App.new(secret: ENV.fetch("LINGARA_APP_SECRET"))
    # Hold per-learner state on `subject`, the same value your events carry.
    app.render do |request|
      (request.slot == "home.side") ? with_note(request) : today_card(STREAKS[request.subject])
    end
    # The button's `action`; it may arrive twice, so make it safe to repeat.
    app.action("done") do |request|
      STREAKS[request.subject] += 1
      today_card(STREAKS[request.subject])
    end
    # lingara:end
  end

  # +builder+ is the Rack::Builder a config.ru is evaluated in.
  def self.serve(builder, app)
    builder.instance_exec do
      # lingara:begin serve
      # config.ru: any Rack server (Puma, Falcon, rackup) runs it. It reads the
      # raw body itself, so mount it where nothing has parsed the JSON first.
      run Lingara::Apps::RackApp.new(app)
      # lingara:end
    end
  end
end
