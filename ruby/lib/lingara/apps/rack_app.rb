# frozen_string_literal: true

module Lingara
  module Apps
    # The Rack adapter (ADR 30.9.26am D2): a Rack-protocol endpoint over an
    # App. The Rack protocol is a callable and a Hash, so the kit depends on
    # no `rack` gem; mount it in any Rack server, Rails or Sinatra.
    #
    #   # config.ru
    #   run Lingara::Apps::RackApp.new(app)
    #
    # It reads `rack.input` itself, as raw bytes and at most 64 KiB + 1 of
    # them, before anything parses it, and hands the env to the library's
    # verifier as the headers, as it is. Mount it where no middleware has
    # already read or decoded the body: the signature covers the exact bytes
    # Lingara sent.
    class RackApp
      def initialize(app)
        raise ArgumentError, "RackApp wraps a Lingara::Apps::App" unless app.is_a?(App)
        @app = app
      end

      def call(env)
        input = env["rack.input"]
        read = ->(limit) { input&.read(limit).to_s }
        status, headers, body = @app.handle(env["REQUEST_METHOD"], env, read)
        [status, headers.dup, body.empty? ? [] : [body]]
      end
    end
  end
end
