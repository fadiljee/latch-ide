/* global document, localStorage, matchMedia */
(function () {
  var root = document.documentElement;
  var style = "tonal";
  var mode = "dark";
  var accent = "#3b82f6";

  try {
    var raw = localStorage.getItem("latch_theme");
    if (raw) {
      var parsed = JSON.parse(raw);
      var state = parsed && parsed.state ? parsed.state : parsed;
      if (state && typeof state === "object") {
        if (
          state.style === "tonal" ||
          state.style === "ledger" ||
          state.style === "bento" ||
          state.style === "glass"
        ) {
          style = state.style;
        }
        if (state.mode === "dark" || state.mode === "light" || state.mode === "system") {
          mode = state.mode;
        }
        if (typeof state.accent === "string" && /^#[0-9a-fA-F]{6}$/.test(state.accent)) {
          accent = state.accent;
        }
      }
    }
  } catch {
    // Gunakan tema bawaan bila storage tidak tersedia atau korup
  }

  var resolvedMode =
    mode === "system"
      ? typeof matchMedia === "function" && matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : mode;

  root.dataset.style = style;
  root.dataset.mode = resolvedMode;
  root.style.setProperty("--accent", accent);
  root.style.setProperty("--accent-soft", "color-mix(in oklab, " + accent + " 16%, transparent)");
})();
