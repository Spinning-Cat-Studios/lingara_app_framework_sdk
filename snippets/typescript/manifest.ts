import { writeFileSync } from "node:fs";

// lingara:begin manifest
import { manifest } from "@lingara/apps";

const json = manifest()
  .defaultLocale("en")
  .name({ en: "Daily five", "zh-Hans": "每日五词" })
  .description("Five words to review, picked from your plan.")
  .renderUrl("https://apps.example.com/lingara/render")
  .slots("home.side", "plans.empty_detail")
  .context("languages", "plan_summary")
  .scopes("plans:read")
  .tutorNote()
  .toJson(); // Throws ManifestError naming the rule an upload would refuse.

writeFileSync("manifest.json", json); // Upload it in the console.
// lingara:end
