# frozen_string_literal: true

# The Ruby kit's fixture app (conformance/CONTRACT.md §F; ADR 30.9.26am D9).
# It is built on the kit's public API and RackApp only, served by WEBrick
# through Rack, so the host's cases judge exactly what an app gets.

require "lingara/apps"
require "rackup"
require "webrick"

def fixture_app
  secrets = ENV.fetch("LINGARA_APPS_CONFORMANCE_SECRETS", "").split(",")
  app = Lingara::Apps::App.new(secret: secrets)
  # A heading naming the slot, then one text per slice received, in order;
  # on home.side, a tutor note too.
  app.render do |request|
    card = request.context.each_with_object(Lingara::Apps.card.heading(request.slot, 1)) { |s, b| b.text(s.kind) }.build
    (request.slot == "home.side") ? Lingara::Apps.reply(card).tutor_note("fixture note") : card
  end
  app.action("inc") { |request| Lingara::Apps.card.progress(0.5, "inc").text(request.card_etag).build }
  app.action("boom") { raise "boom" }
  # 21 list items, which the card builder refuses (list_items).
  app.action("overflow") { Lingara::Apps.card.list((1..21).map { |i| Lingara::Apps.item.text(i.to_s) }).build }
  # 24 legal elements whose reply is over 32 768 bytes (reply_too_large).
  app.action("huge") { 24.times.each_with_object(Lingara::Apps.card) { |_, b| b.text("漢" * 600) }.build }
  app
end

server = WEBrick::HTTPServer.new(
  BindAddress: "127.0.0.1", Port: Integer(ENV.fetch("LINGARA_APPS_CONFORMANCE_PORT", "0")),
  Logger: WEBrick::Log.new($stderr, WEBrick::Log::WARN), AccessLog: []
)
server.mount("/", Rackup::Handler::WEBrick, Lingara::Apps::RackApp.new(fixture_app))
# The host waits for this as the first line of standard output.
$stdout.puts "listening #{server.config[:Port]}"
$stdout.flush
trap("TERM") { server.shutdown }
trap("INT") { server.shutdown }
server.start
