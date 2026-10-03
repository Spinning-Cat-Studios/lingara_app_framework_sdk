import { createServer } from "node:http";

import { Lingara } from "@lingara/api";
import { card, item, lingaraApp, nodeHandler, reply, type AppRenderRequest, type Card } from "@lingara/apps";

// The app's own client-credentials client: it speaks for the app's owner.
const client = new Lingara({
  clientId: process.env.LINGARA_CLIENT_ID!,
  clientSecret: process.env.LINGARA_CLIENT_SECRET!,
});
const streaks = new Map<string, number>();

// lingara:begin card
function todayCard(streak: number): Card {
  return card()
    .heading("Today's five", 1)
    .term({ word: "雨", reading: "yǔ", gloss: "rain", lang: "zh" })
    .list([item.text("Review 3 words"), item.term({ word: "二", reading: "èr" })])
    .button(`Done (${streak})`, "done")
    .build(); // Throws CardLimitError naming the rule the relay would clamp.
}
// lingara:end

// lingara:begin context
async function planLine(request: AppRenderRequest): Promise<string> {
  let line = "";
  for (const slice of request.context) {
    // A slice is present only when the learner agreed to share it.
    if (slice.kind === "languages") line += `${slice.source_lang} → ${slice.target_lang}. `;
    if (slice.kind === "plan_summary") {
      const plan = await client.getLessonPlan({ id: slice.plan_id });
      line += `${plan.title}: ${slice.sets_completed}/${slice.set_count} sets.`;
    }
  }
  return line;
}
// lingara:end

// lingara:begin tutorNote
async function withNote(request: AppRenderRequest) {
  const note = await planLine(request);
  // Plain text, at most 280 characters; the tutor reads it, the learner does not.
  return note === "" ? todayCard(0) : reply(todayCard(0)).tutorNote(note);
}
// lingara:end

// lingara:begin handler
const app = lingaraApp({
  // lgr_whsec_…, from the app's settings; pass two during a rotation.
  secret: process.env.LINGARA_APP_SECRET!,
  // Hold per-learner state on `subject`, the same value your events carry.
  render: (request) => (request.slot === "home.side" ? withNote(request) : todayCard(streaks.get(request.subject) ?? 0)),
  actions: {
    // The button's `action`; may arrive twice, so make it safe to repeat.
    done: (request) => {
      streaks.set(request.subject, (streaks.get(request.subject) ?? 0) + 1);
      return todayCard(streaks.get(request.subject)!);
    },
  },
});
// lingara:end

// lingara:begin serve
// A node:http request listener: it reads the raw body, verifies, dispatches and replies.
createServer(nodeHandler(app)).listen(8787);
// lingara:end
