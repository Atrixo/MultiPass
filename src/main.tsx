import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./index.css";
import { api } from "./lib/api";
import { I18nProvider } from "./lib/i18n";
import { loadSettings, saveSetting } from "./lib/settings";

window.addEventListener("contextmenu", (e) => {
  if (!import.meta.env.DEV) e.preventDefault();
});

if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
  const { installMock } = await import("./dev/mock");
  installMock();
}

const settings = await loadSettings();
if (settings.inventory === undefined) {
  try {
    settings.inventory = await api.defaultInventory();
    saveSetting("inventory", settings.inventory);
  } catch {
    settings.inventory = [];
  }
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <I18nProvider initial={settings.lang}>
      <App settings={settings} />
    </I18nProvider>
  </StrictMode>,
);
