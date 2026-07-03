import { useEffect, useState } from "react";
import type { AppSettings } from "@/lib/api";

export function useTheme(settings: AppSettings | null) {
  const [resolved, setResolved] = useState<"light" | "dark">("light");

  useEffect(() => {
    if (!settings) return;

    const apply = (theme: "light" | "dark") => {
      document.documentElement.classList.toggle("dark", theme === "dark");
      setResolved(theme);
    };

    if (settings.theme === "system") {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      const update = () => apply(mq.matches ? "dark" : "light");
      update();
      mq.addEventListener("change", update);
      return () => mq.removeEventListener("change", update);
    }

    apply(settings.theme === "dark" ? "dark" : "light");
  }, [settings?.theme]);

  return resolved;
}