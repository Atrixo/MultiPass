import { useEffect, useState } from "react";
import { saveSetting, type ThemeMode } from "./settings";

export type { ThemeMode };

export function useTheme(initial: ThemeMode = "system"): [ThemeMode, () => void] {
  const [mode, setMode] = useState<ThemeMode>(initial);
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = mode === "dark" || (mode === "system" && mq.matches);
      document.documentElement.classList.toggle("dark", dark);
    };
    apply();
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [mode]);
  const cycle = () =>
    setMode((m) => {
      const next = m === "system" ? "light" : m === "light" ? "dark" : "system";
      saveSetting("theme", next);
      return next;
    });
  return [mode, cycle];
}
