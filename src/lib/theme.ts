import { useEffect, useState } from "react";

export type ThemeMode = "system" | "light" | "dark";

function load(): ThemeMode {
  try {
    const v = localStorage.getItem("multipass.theme");
    if (v === "light" || v === "dark" || v === "system") return v;
  } catch {
    return "system";
  }
  return "system";
}

export function useTheme(): [ThemeMode, () => void] {
  const [mode, setMode] = useState<ThemeMode>(load);
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = mode === "dark" || (mode === "system" && mq.matches);
      document.documentElement.classList.toggle("dark", dark);
    };
    apply();
    mq.addEventListener("change", apply);
    try {
      localStorage.setItem("multipass.theme", mode);
    } catch {
      return () => mq.removeEventListener("change", apply);
    }
    return () => mq.removeEventListener("change", apply);
  }, [mode]);
  const cycle = () => setMode((m) => (m === "system" ? "light" : m === "light" ? "dark" : "system"));
  return [mode, cycle];
}
