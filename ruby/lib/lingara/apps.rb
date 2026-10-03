# frozen_string_literal: true

# The Lingara app kit for Ruby (ADR 30.9.26am): an app is a server that
# answers Lingara's two signed requests, app.render and app.action, with a
# card. Lingara::Apps::App holds the functions and the core,
# Lingara::Apps::RackApp mounts it, and the card and manifest builders
# refuse at build time what Lingara would otherwise clamp. The signature
# check is the `lingara` library's, the gem's only runtime dependency.
# ../conformance/CONTRACT.md is the behaviour every official kit keeps.

require "json"
require "lingara"

require_relative "apps/version"

Dir[File.join(__dir__, "apps/generated/*.rb")].sort.each { |file| require file }

require_relative "apps/unions"
require_relative "apps/operations"
require_relative "apps/limits"
require_relative "apps/card"
require_relative "apps/manifest"
require_relative "apps/app"
require_relative "apps/rack_app"
