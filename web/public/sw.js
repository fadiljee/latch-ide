/* global self */
// Service Worker minimal Latch PWA (T-225): dukungan install PWA dan Web Push.
self.addEventListener("install", () => {
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener("push", (event) => {
  const data = event.data ? event.data.json() : { title: "Latch", body: "Session update" };
  event.waitUntil(
    self.registration.showNotification(data.title || "Latch", {
      body: data.body || "",
      tag: data.session_id || "latch-event",
    }),
  );
});
