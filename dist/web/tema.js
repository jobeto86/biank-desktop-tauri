(() => {
  const CLAVE = "biank.theme";
  const VALIDOS = /* @__PURE__ */ new Set(["system", "light", "dark"]);
  const desktopBridge = globalThis.biankDesktop;
  if (desktopBridge) document.documentElement.dataset["biankNativeShell"] = "true";
  function leer() {
    try {
      const valor = localStorage.getItem(CLAVE) || "system";
      return VALIDOS.has(valor) ? valor : "system";
    } catch {
      return "system";
    }
  }
  function efectivo(tema = leer()) {
    if (tema === "light" || tema === "dark") return tema;
    return globalThis.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark";
  }
  function avatar(id = "biank", tema = leer()) {
    const normalizado = String(id || "biank").trim().toLowerCase() || "biank";
    const variante = normalizado === "biank" && efectivo(tema) === "dark" ? "-dark" : "";
    const revision = normalizado === "biank" ? "20260922a" : "20260903b";
    return `/avatars/${encodeURIComponent(normalizado)}${variante}.jpg?v=${revision}`;
  }
  function actualizarAvatares(tema = leer()) {
    document.querySelectorAll?.("img[data-agent-avatar]").forEach((img) => {
      const htmlImg = img;
      if ((htmlImg.dataset["agentAvatar"] || "biank") === "biank") return;
      const siguiente = avatar(htmlImg.dataset["agentAvatar"] || "biank", tema);
      if (htmlImg.getAttribute("src") !== siguiente) htmlImg.src = siguiente;
    });
  }
  function aplicar(tema = leer()) {
    const normalizado = VALIDOS.has(tema) ? tema : "system";
    if (normalizado === "system") delete document.documentElement.dataset["theme"];
    else document.documentElement.dataset["theme"] = normalizado;
    const meta = document.querySelector?.('meta[name="theme-color"]');
    const apariencia = efectivo(normalizado);
    if (meta) meta.content = apariencia === "light" ? "#f7f4fc" : "#0b0910";
    void desktopBridge?.titleBar?.setTheme?.(apariencia).catch(() => {
    });
    actualizarAvatares(normalizado);
    return normalizado;
  }
  function guardar(tema) {
    const normalizado = tema && VALIDOS.has(tema) ? tema : "system";
    try {
      if (normalizado === "system") localStorage.removeItem(CLAVE);
      else localStorage.setItem(CLAVE, normalizado);
    } catch {
    }
    return aplicar(normalizado);
  }
  const media = globalThis.matchMedia?.("(prefers-color-scheme: light)");
  media?.addEventListener?.("change", () => {
    if (leer() === "system") aplicar("system");
  });
  const themeController = Object.freeze({ leer, efectivo, aplicar, guardar, avatar });
  globalThis.BiankTheme = themeController;
  aplicar();
})();
