package snippets

import (
	lingaraapps "github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go"
)

func buildCard() (lingaraapps.Card, error) {
	// lingara:begin card
	card, err := lingaraapps.NewCard().
		Heading("Today", 1).
		Term(lingaraapps.Term{Word: "雨", Reading: "yǔ", Gloss: "rain", Lang: "zh"}).
		List(
			lingaraapps.Item.Text("Say it aloud three times."),
			lingaraapps.Item.Term(lingaraapps.Term{Word: "雨天", Gloss: "rainy day", Lang: "zh"}),
		).
		Button("Next", "next").
		Build()
	if err != nil {
		// A *lingaraapps.CardLimitError names the rule the card breaks,
		// such as text_length or list_items. Nothing is cut for you.
		return lingaraapps.Card{}, err
	}
	// lingara:end
	return card, nil
}
