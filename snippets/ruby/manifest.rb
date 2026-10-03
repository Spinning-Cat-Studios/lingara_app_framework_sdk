# frozen_string_literal: true

require "lingara/apps"

module LingaraAppsSnippets
  def self.write_manifest
    # lingara:begin manifest
    manifest = Lingara::Apps.manifest
      .default_locale("en")
      .name("Daily five") # A bare String is the default locale's; or {"en" => …, "zh-Hans" => …}.
      .description("Five words to review, picked from your plan.")
      .render_url("https://apps.example.com/lingara")
      .slots("home.side", "plans.empty_detail")
      .context("languages", "plan_summary")
      .scopes("plans:read")
      .tutor_note
    # Raises Lingara::Apps::ManifestError naming the rule the upload would refuse.
    File.write("manifest.json", manifest.to_json)
    # lingara:end
  end
end
