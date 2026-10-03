# frozen_string_literal: true

require "json"

module Lingara
  module Apps
    # An app (ADR 30.9.26am D4): its verifier, built once, its render
    # function and its action functions, and the framework-neutral core every
    # adapter calls. The signature check is the library's
    # (Lingara::Events::Webhook#verify_signature), never the kit's own.
    #
    #   app = Lingara::Apps::App.new(secret: ENV.fetch("LINGARA_APP_SECRET"))
    #   app.render { |request| Lingara::Apps.card.heading(request.slot).build }
    #   app.action("next") { |request| ... }
    #
    # A function returns a Card, or Lingara::Apps.reply(card).tutor_note(text).
    class App
      JSON_TYPE = {"content-type" => "application/json"}.freeze
      BAD_REQUEST = [400, JSON_TYPE, '{"error":"bad_request"}'].freeze
      HANDLER_FAILED = [500, JSON_TYPE, '{"error":"handler_failed"}'].freeze

      # Each request message the core answers, and how. A unit test holds
      # its keys to the generated Operations::REQUESTS.
      DISPATCH = {
        Operations::RENDER => :render_function,
        Operations::ACTION => :action_function
      }.freeze

      attr_reader :webhook

      # +secret+ is the app's signing secret (lgr_whsec_…), or an Array of
      # them during a rotation. The library's Webhook is built here, so a
      # malformed secret is ArgumentError at startup, not on a request.
      # +logger+ is any object with #warn; the library's own by default.
      def initialize(secret:, logger: Lingara::DefaultLogger.new)
        @webhook = Lingara::Events::Webhook.new(secret)
        @logger = logger
        @render = nil
        @actions = {}
      end

      # Registers the render function: it receives an AppRenderRequest.
      def render(&block)
        raise ArgumentError, "render needs a block" unless block
        @render = block
        self
      end

      # Registers the function for one button +action+: it receives an
      # AppActionRequest whose action_id is +id+.
      def action(id, &block)
        raise ArgumentError, "action needs a block" unless block
        @actions[id.to_s] = block
        self
      end

      # One request, start to finish (AK1–AK4): [status, headers, body].
      # +read+ is called once with the most bytes to read and returns the
      # body read so far, a String.
      def handle(method, headers, read)
        return [405, {}, ""] unless method == "POST"
        body = read.call(Limits::REQUEST_MAX_BYTES + 1).to_s
        return [413, {}, ""] if body.bytesize > Limits::REQUEST_MAX_BYTES
        begin
          @webhook.verify_signature(body, headers)
        rescue Lingara::Events::VerificationError
          return [401, {}, ""]
        end
        request = Decoder.new(@logger).decode(body)
        fn = request && send(DISPATCH.fetch(request.type), request)
        fn ? answer(request, fn) : BAD_REQUEST
      end

      private

      # No render function registered is the app's failure, so a 500.
      def render_function(_request)
        @render || ->(_) { raise "no render function is registered" }
      end

      def action_function(request)
        @actions[request.action_id]
      end

      def answer(request, fn)
        result = fn.call(request)
        wire = result.is_a?(Reply) ? result.to_wire : {"card" => Apps.card_wire(result)}
        [200, JSON_TYPE, Limits.validate_reply(wire)]
      rescue => e
        log(request, reason(e))
        HANDLER_FAILED
      end

      def reason(error)
        return "reply refused: #{error.reason}" if error.is_a?(CardLimitError)
        "raised #{error.class}: #{error.message}"
      end

      # The operation, the install and the reason: never the body, a secret
      # or a signature.
      def log(request, reason)
        @logger.warn("lingara-apps: #{request.type} for install #{request.install_id} failed: #{reason}")
      end
    end

    # The request, decoded leniently into the generated type: an unknown
    # field is ignored, and a context slice of an unknown kind is skipped
    # with a log line, element by element, before the union is read (AK2).
    class Decoder
      # The members every request carries as strings, and an action's two.
      STRINGS = %w[id install_id subject locale].freeze
      ACTION_STRINGS = %w[action_id card_etag].freeze

      def initialize(logger)
        @logger = logger
      end

      # The generated request, or nil (a 400).
      def decode(body)
        raw = parse(body)
        return nil unless raw.is_a?(Hash) && App::DISPATCH.key?(raw["type"])
        return nil unless ManifestRules::SLOT_NAMES.include?(raw["slot"]) && strings?(raw)
        context = context(raw["context"])
        return nil unless context
        request = Apps.const_get(Operations::REQUESTS.fetch(raw["type"])).build_from_hash(raw.merge("context" => []))
        request.context = context
        request
      rescue
        nil
      end

      private

      def strings?(raw)
        names = (raw["type"] == Operations::ACTION) ? STRINGS + ACTION_STRINGS : STRINGS
        names.all? { |name| raw[name].is_a?(String) }
      end

      def parse(body)
        text = body.dup.force_encoding(Encoding::UTF_8)
        text.valid_encoding? ? JSON.parse(text) : nil
      rescue JSON::ParserError
        nil
      end

      def context(raw)
        return nil unless raw.is_a?(Array)
        raw.each_with_object([]) do |element, slices|
          raise ArgumentError, "a context slice is not an object" unless element.is_a?(Hash) && element["kind"].is_a?(String)
          slice = ContextSlice.decode(element)
          slice ? slices << slice : skip(element["kind"])
        end
      end

      def skip(kind)
        @logger.warn("lingara-apps: skipped a context slice of unknown kind #{kind[0, 64].inspect}")
      end
    end
  end
end
