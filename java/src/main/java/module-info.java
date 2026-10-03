/**
 * The Lingara app kit for Java (ADR 30.9.26am D2).
 *
 * <p>{@code com.getlingara.apps} is the app, its builders, its limits and the {@code
 * jdk.httpserver} adapter; {@code com.getlingara.apps.model} is the app types, generated from the
 * app view. The signature verifier is the Lingara library's, so the library is the one module the
 * kit requires. {@code jdk.httpserver} is required {@code static}: the core loads on a runtime
 * image without it.
 */
module com.getlingara.apps {
  requires transitive com.getlingara.client;
  requires static jdk.httpserver;

  exports com.getlingara.apps;
  exports com.getlingara.apps.model;

  opens com.getlingara.apps.model to
      com.fasterxml.jackson.databind;
}
