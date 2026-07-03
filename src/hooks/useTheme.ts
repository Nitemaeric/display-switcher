import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { AppSettings } from "@/lib/api";

type ResolvedTheme = "light" | "dark";

async function resolveSystemTheme(): Promise<ResolvedTheme> {
  try {
    const resolved = await invoke<string>("resolve_theme_setting", {
      theme: "system",
    });
    return resolved === "dark" ? "dark" : "light";
  } catch {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
}

export function useTheme(settings: AppSettings | null) {
  const [resolved, setResolved] = useState<ResolvedTheme>("light");

  useEffect(() => {
    if (!settings) return;

    let cancelled = false;

    const apply = (resolvedTheme: ResolvedTheme, preference: string) => {
      if (cancelled) return;
      document.documentElement.classList.toggle(
        "dark",
        resolvedTheme === "dark",
      );
      setResolved(resolvedTheme);
      invoke("sync_window_chrome", { theme: preference }).catch(() => {});
    };

    if (settings.theme === "system") {
      const update = async () => {
        apply(await resolveSystemTheme(), "system");
      };

      void update();

      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      mq.addEventListener("change", update);

      const unlisten = getCurrentWindow()
        .onThemeChanged(() => {
          void update();
        })
        .catch(() => null);

      return () => {
        cancelled = true;
        mq.removeEventListener("change", update);
        unlisten.then((fn) => fn?.());
      };
    }

    apply(settings.theme === "dark" ? "dark" : "light", settings.theme);

    return () => {
      cancelled = true;
    };
  }, [settings?.theme]);

  return resolved;
}