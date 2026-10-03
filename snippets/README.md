# Snippets

Each kit fills `snippets/<lang>/` with the examples the public docs show, in
the region format the client libraries' snippets use: a region opens with a
`lingara:begin <key>` comment line and closes with `lingara:end`, and the
docs site reads each region by its key. A kit's snippets compile and run in
its own test lane, so a docs example cannot drift from the kit.

Until a kit lands, its `snippets/<lang>/` does not exist and does not ship.
