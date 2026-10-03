# frozen_string_literal: true

# The gem `lingara-apps` (ADR 30.9.26am D1, D2). Its one runtime dependency
# is the `lingara` library, from the library floor (LIBRARY_FLOOR at the
# repository root, in RubyGems' `.pre.` spelling) up to the next minor.
# require_paths stays at its default and no bindir is set, so the publisher
# reads this file without running it.

require_relative "lib/lingara/apps/version"

Gem::Specification.new do |spec|
  spec.name = "lingara-apps"
  spec.version = Lingara::Apps::GEM_VERSION
  spec.authors = ["Spinning Cat Studios"]
  spec.summary = "The official Ruby kit for building Lingara apps"
  spec.description = "Answer Lingara's signed app.render and app.action requests with a card: " \
    "verification by the lingara library, a Rack endpoint, and card and manifest builders that refuse what Lingara would clamp."
  spec.homepage = "https://github.com/Spinning-Cat-Studios/lingara_app_framework_sdk"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.3"
  spec.metadata = {
    "source_code_uri" => "https://github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/tree/main/ruby",
    "rubygems_mfa_required" => "true"
  }
  spec.files = Dir[File.join(__dir__, "{lib/**/*.rb,sig/**/*.rbs}")].map { |path| path.delete_prefix("#{__dir__}/") } +
    %w[README.md LICENSE]

  spec.add_dependency "lingara", ">= 0.1.0.pre.alpha.10", "< 0.2"
end
