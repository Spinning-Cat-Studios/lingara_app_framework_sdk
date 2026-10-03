# frozen_string_literal: true

require "json"

module Lingara
  module Apps
    # A manifest the upload would refuse, with the first rule it breaks.
    # #rule is one of ManifestRules::RULES.
    class ManifestError < StandardError
      attr_reader :rule

      def initialize(rule)
        @rule = rule
        super("the manifest breaks the #{rule} rule")
      end
    end

    # The manifest's rules (ADR 30.9.26am D5), hand-written against the
    # manifest's wire form, which is in no spec the view reads. Each refuses
    # what the upload would refuse or normalise away where a kit can know it,
    # in the order conformance/vectors/manifest.json describes; a refusal
    # names the first broken rule.
    module ManifestRules
      NAME_MAX_CHARS = 40
      DESCRIPTION_MAX_CHARS = 280
      STORED_MAX_BYTES = 65_536

      RULES = %i[manifest_version default_locale name description render_url slots context duplicate too_large].freeze

      # The slots a manifest may name: the generated AppSlotName's values,
      # without the generator's unknown-value placeholder.
      SLOT_NAMES = (AppSlotName.all_vars - [AppSlotName::UNKNOWN_DEFAULT_OPEN_API]).freeze

      # Zero-width characters, beyond the card rules' controls and bidi marks.
      ZERO_WIDTH = /[\u200B-\u200F\uFEFF]/
      RENDER_URL = %r{\Ahttps://[^/?#]}i

      module_function

      def localized?(value, max)
        return false unless value.is_a?(Hash) && !value.empty?
        value.each_value.all? do |v|
          next false unless v.is_a?(String)
          trimmed = Limits.trim(v)
          trimmed.length.between?(1, max) && !Limits.control?(trimmed, false) && !trimmed.match?(ZERO_WIDTH)
        end
      end

      def default_locale?(m)
        locale = m["default_locale"]
        return false unless locale.is_a?(String) && !locale.empty?
        [m["name"], m["description"]].all? { |map| !map.is_a?(Hash) || map.key?(locale) }
      end

      # A list of allowed strings; absent passes only when +optional+.
      def within?(m, key, allowed, optional)
        return optional unless m.key?(key)
        value = m[key]
        return false unless value.is_a?(Array)
        return false if value.empty? && !optional
        value.all? { |v| v.is_a?(String) && allowed.include?(v) }
      end

      def duplicate?(m)
        %w[slots context scopes].any? { |k| m[k].is_a?(Array) && m[k].uniq.size != m[k].size }
      end

      # The compact UTF-8 JSON of the seven stored keys, defaults filled.
      def stored_bytes(m)
        stored = {
          "default_locale" => m["default_locale"], "name" => m["name"], "description" => m["description"],
          "slots" => m["slots"], "context" => m["context"] || [], "scopes" => m["scopes"] || [],
          "tutor_note" => m["tutor_note"].nil? ? false : m["tutor_note"]
        }
        JSON.generate(stored).bytesize
      end

      CHECKS = {
        manifest_version: ->(m) { m["manifest_version"].is_a?(Integer) && m["manifest_version"] == 1 },
        default_locale: ->(m) { default_locale?(m) },
        name: ->(m) { localized?(m["name"], NAME_MAX_CHARS) },
        description: ->(m) { localized?(m["description"], DESCRIPTION_MAX_CHARS) },
        render_url: ->(m) { m["render_url"].is_a?(String) && m["render_url"].match?(RENDER_URL) },
        slots: ->(m) { within?(m, "slots", SLOT_NAMES, false) },
        context: ->(m) { within?(m, "context", CONTEXT_SLICE_KINDS, true) },
        duplicate: ->(m) { !duplicate?(m) },
        too_large: ->(m) { stored_bytes(m) <= STORED_MAX_BYTES }
      }.freeze

      # The first rule +manifest+ breaks, or nil. Takes any JSON value.
      def first_broken(manifest)
        m = manifest.is_a?(Hash) ? manifest : {}
        RULES.find { |rule| !CHECKS.fetch(rule).call(m) }
      end
    end

    # A fluent manifest. #build validates and returns the wire form (a Hash);
    # #to_json writes the upload.
    #
    #   Lingara::Apps.manifest.default_locale("en").name("Daily five").description("Five words.")
    #     .render_url("https://apps.example.com/lingara").slots("home.side").build
    class ManifestBuilder
      def initialize
        @default_locale = ""
        @name = ""
        @description = ""
        @render_url = ""
        @slots = []
        @context = []
        @scopes = []
        @tutor_note = false
        @listed = false
      end

      def default_locale(locale) = set(:@default_locale, locale)

      # A locale map, or a bare String meaning the default locale's.
      def name(name) = set(:@name, name)

      # A locale map, or a bare String meaning the default locale's.
      def description(description) = set(:@description, description)

      def render_url(url) = set(:@render_url, url)

      # AppSlotName values.
      def slots(*slots) = set(:@slots, slots)

      # CONTEXT_SLICE_KINDS values.
      def context(*kinds) = set(:@context, kinds)

      def scopes(*scopes) = set(:@scopes, scopes)

      # Whether the app's replies may carry a tutor note.
      def tutor_note(enabled = true) = set(:@tutor_note, enabled)

      # Whether the app appears in the catalogue; false unless set.
      def listed(listed = true) = set(:@listed, listed)

      # The manifest, or ManifestError naming the first rule it breaks.
      def build
        manifest = {
          "manifest_version" => 1, "default_locale" => @default_locale, "name" => localized(@name),
          "description" => localized(@description), "render_url" => @render_url, "slots" => @slots.dup,
          "context" => @context.dup, "scopes" => @scopes.dup, "tutor_note" => @tutor_note, "listed" => @listed
        }
        Apps.validate_manifest(manifest)
        manifest
      end

      # The validated manifest as the JSON the console accepts as an upload.
      def to_json(*)
        "#{JSON.pretty_generate(build)}\n"
      end

      private

      def set(ivar, value)
        instance_variable_set(ivar, value)
        self
      end

      def localized(value)
        value.is_a?(String) ? {@default_locale => value} : value
      end
    end

    module_function

    # Starts a manifest.
    def manifest
      ManifestBuilder.new
    end

    # Raises ManifestError naming the first rule +manifest+ (a Hash with
    # String keys, as JSON.parse gives) breaks; nil when it keeps them all.
    def validate_manifest(manifest)
      rule = ManifestRules.first_broken(manifest)
      raise ManifestError, rule if rule
    end
  end
end
