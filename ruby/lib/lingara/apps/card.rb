# frozen_string_literal: true

module Lingara
  module Apps
    # List items for CardBuilder#list: ListItemText and ListItemTerm.
    module Item
      module_function

      def text(text, lang: nil)
        ListItemText.new(text: text, lang: lang)
      end

      def term(word:, reading: nil, gloss: nil, lang: nil)
        ListItemTerm.new(word: word, reading: reading, gloss: gloss, lang: lang)
      end
    end

    # A fluent card (ADR 30.9.26am D5): one method per element, each taking
    # its generated arm's closed member set, and #build checks the card
    # rules. A kit refuses what the relay would clamp, so the clamp never
    # fires on a kit-built card.
    #
    #   Lingara::Apps.card.heading("Today").term(word: "雨", reading: "yǔ").button("Next", "next").build
    class CardBuilder
      def initialize
        @elements = []
      end

      def heading(text, level = 1)
        push(CardElementHeading.new(text: text, level: level))
      end

      def text(text, lang: nil)
        push(CardElementText.new(text: text, lang: lang))
      end

      def term(word:, reading: nil, gloss: nil, lang: nil)
        push(CardElementTerm.new(word: word, reading: reading, gloss: gloss, lang: lang))
      end

      # +items+ are Item.text and Item.term values.
      def list(items)
        push(CardElementList.new(items: items.to_a.dup.freeze))
      end

      def progress(value, label)
        push(CardElementProgress.new(value: value, label: label))
      end

      def divider
        push(CardElementDivider.new)
      end

      # +action+ comes back as the request's action_id; +style+ is a
      # ButtonStyle value ("primary" or "secondary").
      def button(label, action, style: nil)
        push(CardElementButton.new(label: label, action: action, style: style))
      end

      def link(label, url)
        push(CardElementLink.new(label: label, url: url))
      end

      # The generated Card, or CardLimitError naming the first card rule it
      # breaks.
      def build
        card = Card.new(elements: @elements.dup.freeze)
        Limits.check_card(Apps.card_wire(card))
        card
      end

      private

      def push(element)
        @elements << element
        self
      end
    end

    # A card and, optionally, a tutor note: what a render or action function
    # may return instead of a bare card.
    class Reply
      attr_reader :card

      def initialize(card)
        raise ArgumentError, "a reply carries a Lingara::Apps::Card" unless card.is_a?(Card)
        @card = card
        @tutor_note = nil
      end

      # Plain text, at most 280 characters once `\n` becomes a space and the
      # ends are trimmed; CardLimitError otherwise.
      def tutor_note(text = nil)
        return @tutor_note if text.nil?
        Limits.check_tutor_note(text)
        @tutor_note = text
        self
      end

      # The wire form, {card, tutor_note?}: an absent note is omitted.
      def to_wire
        wire = {"card" => Apps.card_wire(@card)}
        wire["tutor_note"] = @tutor_note unless @tutor_note.nil?
        wire
      end
    end

    module_function

    # Starts a card.
    def card
      CardBuilder.new
    end

    # The list-item constructors: `Lingara::Apps.item.text("one")`.
    def item
      Item
    end

    # Wraps a built card so a tutor note can ride with it.
    def reply(card)
      Reply.new(card)
    end

    # A generated Card's wire form, {elements: [...]}, each arm in its own.
    def card_wire(card)
      {"elements" => card.elements.map { |e| e.respond_to?(:to_wire) ? e.to_wire : e }}
    end

    # The relay's cut, when asked for: Limits.truncate.
    def truncate(text, limit)
      Limits.truncate(text, limit)
    end

    # Limits.validate_reply: the bytes to send, or CardLimitError.
    def validate_reply(reply)
      Limits.validate_reply(reply)
    end
  end
end
