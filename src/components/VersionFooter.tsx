import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";

export function VersionFooter() {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(null));
  }, []);

  return (
    <footer className="shrink-0 px-3 py-3 text-center text-xs text-[var(--color-muted)]">
      Display Switcher{version ? ` v${version}` : ""}
    </footer>
  );
}
