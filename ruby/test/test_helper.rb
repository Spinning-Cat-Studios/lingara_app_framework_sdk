# frozen_string_literal: true

require "json"
require "minitest/autorun"
require "openssl"
require "stringio"
require "lingara/apps"

# What the kit's tests share: the repository's paths, a logger that keeps
# what it is told, and a request signed as Lingara signs one (Standard
# Webhooks: HMAC-SHA256 over `id.timestamp.body`, keyed by the base64 after
# the secret's lgr_whsec_ prefix). Nothing here reaches a network.
module KitTest
  RUBY_DIR = File.expand_path("..", __dir__)
  ROOT = File.expand_path("..", RUBY_DIR)
  KEY = "unit-app-secret-0000000000000001"
  SECRET = "lgr_whsec_#{[KEY].pack("m0")}"

  # A logger with #warn, as the library's DefaultLogger has.
  class Recorder
    attr_reader :lines

    def initialize
      @lines = []
    end

    def warn(message)
      @lines << message
    end
  end

  module_function

  def vectors(name)
    JSON.parse(File.read(File.join(ROOT, "conformance/vectors", name))).fetch("vectors")
  end

  # A Rack env carrying +body+, signed now with KEY.
  def signed_env(body, id: "lgr_msg_#{"0" * 32}", method: "POST")
    timestamp = Time.now.to_i.to_s
    mac = OpenSSL::HMAC.digest("SHA256", KEY, "#{id}.#{timestamp}.#{body}")
    {"REQUEST_METHOD" => method, "rack.input" => StringIO.new(body.b), "CONTENT_TYPE" => "application/json",
     "HTTP_WEBHOOK_ID" => id, "HTTP_WEBHOOK_TIMESTAMP" => timestamp, "HTTP_WEBHOOK_SIGNATURE" => "v1,#{[mac].pack("m0")}"}
  end

  def request(type, **extra)
    {type: type, id: "lgr_msg_#{"0" * 32}", install_id: "6f9c1d2e-3a4b-4c5d-8e7f-a0b1c2d3e4f5", subject: "lgr_sub_unit",
     slot: "home.side", locale: "en", context: []}.merge(extra)
  end
end
