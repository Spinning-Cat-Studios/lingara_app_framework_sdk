package com.getlingara.apps.conformance;

import com.getlingara.apps.Card;
import com.getlingara.apps.Item;
import com.getlingara.apps.JdkHttpHandler;
import com.getlingara.apps.LingaraApp;
import com.getlingara.apps.Reply;
import com.getlingara.apps.model.AppRenderRequest;
import com.getlingara.apps.model.AppSlotName;
import com.getlingara.apps.model.ContextSlice;
import com.getlingara.apps.model.ContextSliceKind;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.util.ArrayList;
import java.util.List;

/**
 * The contract's fixture app (§F) on the Java kit: every reply is exact, because the host judges
 * them JSON-equal. It reads its secrets from {@code LINGARA_APPS_CONFORMANCE_SECRETS} and its port
 * from {@code LINGARA_APPS_CONFORMANCE_PORT}, listens on 127.0.0.1 and prints {@code listening
 * <port>} as its first line.
 */
public final class Fixture {
  private Fixture() {}

  /**
   * Starts the fixture app.
   *
   * @param args unused
   * @throws IOException when the server cannot bind
   */
  public static void main(String[] args) throws IOException {
    String[] secrets = System.getenv("LINGARA_APPS_CONFORMANCE_SECRETS").split(",");
    String port = System.getenv().getOrDefault("LINGARA_APPS_CONFORMANCE_PORT", "0");
    LingaraApp app =
        LingaraApp.builder()
            .secrets(secrets)
            .render(Fixture::render)
            .action("inc", r -> Card.card().progress(0.5, "inc").text(r.getCardEtag()).build())
            .action(
                "boom",
                r -> {
                  throw new IllegalStateException("boom");
                })
            .action("overflow", r -> overflow())
            .action("huge", r -> huge())
            .build();
    InetSocketAddress at =
        new InetSocketAddress(InetAddress.getLoopbackAddress(), Integer.parseInt(port));
    HttpServer server = HttpServer.create(at, 0);
    server.createContext("/", JdkHttpHandler.of(app));
    server.start();
    System.out.println("listening " + server.getAddress().getPort());
    System.out.flush();
  }

  private static Reply render(AppRenderRequest request) {
    Card.Builder card = Card.card().heading(request.getSlot().getValue(), 1);
    for (ContextSlice slice : request.getContext()) {
      card.text(ContextSliceKind.of(slice).getValue());
    }
    if (request.getSlot() == AppSlotName.HOME_SIDE) {
      return Reply.reply(card.build()).tutorNote("fixture note");
    }
    return card.build();
  }

  /** One list of 21 items: the public builder refuses it, so the action raises. */
  private static Reply overflow() {
    List<Item> items = new ArrayList<>();
    for (int i = 1; i <= 21; i++) {
      items.add(Item.text(Integer.toString(i)));
    }
    return Card.card().list(items).build();
  }

  /** 24 texts of 600 漢: every card rule passes, and the encoded reply is over 32 KiB. */
  private static Reply huge() {
    Card.Builder card = Card.card();
    for (int i = 0; i < 24; i++) {
      card.text("漢".repeat(600));
    }
    return card.build();
  }
}
