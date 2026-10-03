package snippets;

// lingara:begin manifest
import com.getlingara.apps.Manifest;
import com.getlingara.apps.model.AppSlotName;
import com.getlingara.apps.model.ContextSliceKind;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;

// lingara:end

/** The documentation site's manifest example. */
public final class ManifestSnippet {
  private ManifestSnippet() {}

  static void write() throws IOException {
    // lingara:begin manifest
    String json =
        Manifest.manifest()
            .defaultLocale("en")
            .name("Daily five")
            .description("Five words to review, picked from your plan.")
            .renderUrl("https://apps.example.com/lingara/render")
            .slots(AppSlotName.HOME_SIDE)
            .context(ContextSliceKind.LANGUAGES, ContextSliceKind.PLAN_SUMMARY)
            .scopes("plans:read")
            .tutorNote(true)
            .build() // refuses what the upload would, naming the rule
            .toJson();
    // Upload manifest.json in the console, beside the app's icon.
    Files.writeString(Path.of("manifest.json"), json);
    // lingara:end
  }
}
