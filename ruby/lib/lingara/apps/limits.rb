# frozen_string_literal: true

require "json"

module Lingara
  module Apps
    # A card or reply the relay would clamp, refused with the first rule it
    # breaks. #reason is one of Limits::REASONS.
    class CardLimitError < StandardError
      attr_reader :reason

      def initialize(reason)
        @reason = reason
        super("the card breaks the #{reason} rule")
      end
    end

    # The card rules (ADR 30.9.26am D5): the relay's clamp restated as
    # refusals, held to conformance/vectors/card-limits.json and to the
    # contract's reference validator. Every limit is a constant here and
    # nowhere else, because the view carries none of them: they are
    # post-parse clamps.
    #
    # A "character" is a Unicode scalar value (String#length on UTF-8), never
    # a grapheme. "Empty" is empty after trimming Unicode White_Space, which
    # String#strip does not do (it trims ASCII only). Patterns match the
    # whole string (\A…\z), never one line of it as ^…$ would.
    module Limits
      # The largest reply body the relay reads, in bytes.
      REPLY_MAX_BYTES = 32_768
      # The largest request body a kit reads, in bytes.
      REQUEST_MAX_BYTES = 65_536
      MAX_ELEMENTS = 24
      MAX_BUTTONS = 4
      MAX_LIST_ITEMS = 20
      TUTOR_NOTE_MAX_CHARS = 280
      MAX_URL_BYTES = 2048

      HEADING_MAX_CHARS = 80
      TEXT_MAX_CHARS = 600
      TERM_WORD_MAX_CHARS = 60
      TERM_READING_MAX_CHARS = 120
      TERM_GLOSS_MAX_CHARS = 160
      PROGRESS_LABEL_MAX_CHARS = 60
      BUTTON_LABEL_MAX_CHARS = 32
      LINK_LABEL_MAX_CHARS = 60

      LANG_PATTERN = /\A[A-Za-z]{2,3}(?:-[A-Za-z0-9]{2,8}){0,3}\z/
      ACTION_PATTERN = /\A[A-Za-z0-9_.:-]{1,64}\z/

      # The refusal reasons, in the contract's order: a reply's reason is the
      # first it breaks.
      REASONS = %i[control_chars text_length heading_level progress_range lang link button_action buttons list_items
        empty_element elements empty_card tutor_note_length reply_too_large].freeze

      # C0 and C1 controls, and the bidi overrides and isolates; the second
      # leaves out `\n`.
      CONTROL = /[\u0000-\u001F\u007F-\u009F\u202A-\u202E\u2066-\u2069]/
      CONTROL_BUT_NEWLINE = /[\u0000-\u0009\u000B-\u001F\u007F-\u009F\u202A-\u202E\u2066-\u2069]/
      EDGES = /\A\p{White_Space}+|\p{White_Space}+\z/

      # One text field's rule: its limit, whether `\n` is legal, whether it
      # must be non-empty.
      Field = Data.define(:limit, :newline, :required)
      HEADING = Field.new(HEADING_MAX_CHARS, false, true)
      TEXT = Field.new(TEXT_MAX_CHARS, true, true)
      WORD = Field.new(TERM_WORD_MAX_CHARS, false, true)
      READING = Field.new(TERM_READING_MAX_CHARS, false, false)
      GLOSS = Field.new(TERM_GLOSS_MAX_CHARS, false, false)
      PROGRESS_LABEL = Field.new(PROGRESS_LABEL_MAX_CHARS, false, true)
      BUTTON_LABEL = Field.new(BUTTON_LABEL_MAX_CHARS, false, true)
      LINK_LABEL = Field.new(LINK_LABEL_MAX_CHARS, false, true)
      # `lang`, `button.action` and `link.url`: no length limit of their own.
      RAW = Field.new(Float::INFINITY, false, false)

      # The rules a reply breaks, collected; the reason is the first in
      # REASONS order, as the reference validator's BTreeSet minimum is.
      class Found
        def initialize
          @broken = []
        end

        def flag(reason, broken)
          @broken << reason if broken
        end

        def raise_first
          reason = REASONS.find { |r| @broken.include?(r) }
          raise CardLimitError, reason if reason
        end

        def text(value, rule)
          unless value.is_a?(String)
            flag(:empty_element, rule.required)
            return nil
          end
          flag(:control_chars, Limits.control?(value, rule.newline))
          flag(:text_length, value.length > rule.limit)
          flag(:empty_element, rule.required && Limits.trim(value).empty?)
          value
        end

        def lang(value)
          tag = text(value, RAW)
          flag(:lang, !tag.match?(LANG_PATTERN)) if tag
        end

        def item(item)
          case item["type"]
          when "text"
            text(item["text"], TEXT)
            lang(item["lang"])
          when "term"
            text(item["word"], WORD)
            text(item["reading"], READING) unless item["reading"].nil?
            text(item["gloss"], GLOSS) unless item["gloss"].nil?
            lang(item["lang"])
          end
        end

        def element(e)
          case e["type"]
          when "heading"
            text(e["text"], HEADING)
            flag(:heading_level, !(e["level"].is_a?(Integer) && [1, 2].include?(e["level"])))
          when "text", "term" then item(e)
          when "list" then list(e)
          when "progress"
            flag(:progress_range, !(e["value"].is_a?(Numeric) && e["value"].between?(0, 1)))
            text(e["label"], PROGRESS_LABEL)
          when "button"
            text(e["label"], BUTTON_LABEL)
            flag(:button_action, !text(e["action"], RAW).to_s.match?(ACTION_PATTERN))
          when "link"
            text(e["label"], LINK_LABEL)
            flag(:link, !Limits.safe_link?(text(e["url"], RAW).to_s))
          end
        end

        def list(e)
          items = e["items"].is_a?(Array) ? e["items"] : []
          items.each { |i| item(i.is_a?(Hash) ? i : {}) }
          flag(:list_items, items.empty? || items.size > MAX_LIST_ITEMS)
        end

        def card(card)
          raw = card.is_a?(Hash) ? card["elements"] : nil
          elements = Array(raw.is_a?(Array) ? raw : nil).map { |e| e.is_a?(Hash) ? e : {} }
          elements.each { |e| element(e) }
          flag(:buttons, elements.count { |e| e["type"] == "button" } > MAX_BUTTONS)
          flag(:elements, elements.size > MAX_ELEMENTS)
          flag(:empty_card, elements.empty?)
        end

        def tutor_note(note)
          return unless note.is_a?(String)
          flag(:control_chars, Limits.control?(note, true))
          # The relay turns `\n` into a space and trims before it counts.
          flag(:tutor_note_length, Limits.trim(note.tr("\n", " ")).length > TUTOR_NOTE_MAX_CHARS)
        end
      end

      module_function

      # Trims Unicode White_Space from both ends.
      def trim(text)
        text.gsub(EDGES, "")
      end

      # Whether `text` holds a refused character, `\n` aside when `newline`.
      def control?(text, newline)
        text.match?(newline ? CONTROL_BUT_NEWLINE : CONTROL)
      end

      # The relay's cut, never applied implicitly: over `limit` scalar values,
      # the first `limit − 1` and `…`; otherwise unchanged.
      def truncate(text, limit)
        return text if text.length <= limit
        "#{text[0, [limit - 1, 0].max]}…"
      end

      # `https`, no user information, a host name (never an IP address), at
      # most MAX_URL_BYTES. Read as a WHATWG parser reads a special URL: a
      # host whose last label is a number is IPv4 (127.1, 0x7f.1).
      def safe_link?(raw)
        return false if raw.bytesize > MAX_URL_BYTES
        authority = raw[%r{\Ahttps://([^/?#\\]*)}i, 1]
        return false if authority.nil? || authority.include?("@")
        host, port = authority.match(/\A(.*?)(?::(\d*))?\z/m).captures
        return false if port && !port.empty? && port.to_i > 65_535
        !host.empty? && !host.match?(%r{[\u0000\t\n\r #/:<>?@\[\\\]^|%]}) && !ends_in_number?(host)
      end

      def ends_in_number?(host)
        last = host.delete_suffix(".").split(".").last.to_s.downcase
        return false if last.empty?
        last.match?(/\A(?:\d+|0x\h*)\z/)
      end

      # Every card rule, the tutor note, and the size of the reply encoded
      # exactly as it will be sent: compact JSON, raw UTF-8, `/` unescaped,
      # an absent tutor_note omitted. Returns those bytes, or raises
      # CardLimitError with the first reason. Takes the generic JSON shape
      # (a Hash with String keys), so a vector feeds it as it is.
      def validate_reply(reply)
        found = Found.new
        reply = {} unless reply.is_a?(Hash)
        found.card(reply["card"])
        found.tutor_note(reply["tutor_note"])
        found.raise_first
        bytes = JSON.generate(reply)
        raise CardLimitError, :reply_too_large if bytes.bytesize > REPLY_MAX_BYTES
        bytes
      end

      # The card rules alone (reasons 1–12): what CardBuilder#build runs.
      def check_card(card)
        Found.new.tap { |f| f.card(card) }.raise_first
      end

      # The tutor note's rules alone: what Reply#tutor_note runs.
      def check_tutor_note(note)
        Found.new.tap { |f| f.tutor_note(note) }.raise_first
      end
    end
  end
end
