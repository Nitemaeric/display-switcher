import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Monitor, Plus, Settings, Trash2 } from "lucide-react";
import { toast, Toaster } from "sonner";
import {
  api,
  type AppConfig,
  type DisplayGroup,
  type DisplayInfo,
} from "@/lib/api";
import { useTheme } from "@/hooks/useTheme";
import { Button } from "@/components/ui/button";
import { GroupEditor } from "@/components/GroupEditor";
import { TelemetryPanel } from "@/components/TelemetryPanel";
import { OnboardingWizard } from "@/components/OnboardingWizard";

type Tab = "groups" | "settings";

function App() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [displays, setDisplays] = useState<DisplayInfo[]>([]);
  const [tab, setTab] = useState<Tab>("groups");
  const [editing, setEditing] = useState<DisplayGroup | null>(null);
  const [builtinActions, setBuiltinActions] = useState<[string, string][]>([]);
  const [gamepadButtons, setGamepadButtons] = useState<string[]>([]);

  useTheme(config?.settings ?? null);

  const reload = useCallback(async () => {
    const [cfg, disp, actions, buttons] = await Promise.all([
      api.getConfig(),
      api.listDisplays(),
      api.getBuiltinActions(),
      api.getGamepadButtons(),
    ]);
    setConfig(cfg);
    setDisplays(disp);
    setBuiltinActions(actions);
    setGamepadButtons(buttons);
  }, []);

  useEffect(() => {
    reload().catch((e) => toast.error(String(e)));
  }, [reload]);

  useEffect(() => {
    const unlisten = listen<string>("activate-group", async (event) => {
      try {
        const record = await api.activateGroup(event.payload, "hotkey");
        toast.success(`${record.group_name} activated (${record.display_apply_ms}ms)`);
      } catch (e) {
        toast.error(String(e));
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const saveSettings = async (partial: Partial<AppConfig["settings"]>) => {
    if (!config) return;
    const next = { ...config, settings: { ...config.settings, ...partial } };
    await api.saveConfig(next);
    setConfig(next);
    toast.success("Settings saved");
  };

  const handleUpdateGroup = async (group: DisplayGroup) => {
    await api.updateGroup(group);
    await reload();
  };

  const handleCreateGroup = async () => {
    const group = await api.createGroup("New Group");
    setEditing(group);
    await reload();
  };

  const handleDeleteGroup = async (id: string) => {
    await api.deleteGroup(id);
    await reload();
    toast.success("Group deleted");
  };

  if (!config) {
    return (
      <div className="flex min-h-screen items-center justify-center text-[var(--color-muted)]">
        Loading…
      </div>
    );
  }

  if (!config.onboarding_complete) {
    return (
      <>
        <OnboardingWizard displays={displays} onComplete={reload} />
        <Toaster richColors position="bottom-right" />
      </>
    );
  }

  if (editing) {
    return (
      <div className="min-h-screen p-3">
        <div className="mx-auto max-w-2xl">
          <h2 className="mb-3 text-xl font-semibold">Edit group</h2>
          <GroupEditor
            group={editing}
            displays={displays}
            builtinActions={builtinActions}
            gamepadButtons={gamepadButtons}
            onSave={handleUpdateGroup}
            onClose={() => setEditing(null)}
          />
        </div>
        <Toaster richColors position="bottom-right" />
      </div>
    );
  }

  return (
    <div className="min-h-screen">
      <header className="border-b border-[var(--color-card-border)] px-3 py-3">
        <div className="mx-auto flex max-w-4xl items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-[var(--color-accent)] text-white">
              <Monitor size={20} />
            </div>
            <div>
              <h1 className="text-lg font-semibold">Display Switcher</h1>
              <p className="text-sm text-[var(--color-muted)]">
                {displays.length} display{displays.length === 1 ? "" : "s"} connected
              </p>
            </div>
          </div>
          <nav className="flex items-center gap-1 rounded-lg border border-[var(--color-card-border)] p-1">
            <TabButton active={tab === "groups"} onClick={() => setTab("groups")}>
              Groups
            </TabButton>
            <TabButton
              active={tab === "settings"}
              onClick={() => setTab("settings")}
            >
              <Settings size={14} /> Settings
            </TabButton>
          </nav>
        </div>
      </header>

      <main className="px-3 py-3">
        <div className="mx-auto max-w-4xl">
        {tab === "groups" ? (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <h2 className="text-base font-medium">Display groups</h2>
              <Button className="shrink-0" onClick={handleCreateGroup}>
                <Plus size={16} /> Add group
              </Button>
            </div>

            {config.groups.length === 0 ? (
              <div className="rounded-xl border border-dashed border-[var(--color-card-border)] py-3 text-center text-[var(--color-muted)]">
                No groups yet. Create one to get started.
              </div>
            ) : (
              <div className="grid gap-3">
                {config.groups.map((group) => {
                  const canActivate = group.display_ids.length > 0;
                  return (
                  <div
                    key={group.id}
                    role="button"
                    tabIndex={0}
                    className="flex cursor-pointer items-center justify-between rounded-xl border border-[var(--color-card-border)] bg-[var(--color-card)] p-3 transition-colors hover:bg-black/5 dark:hover:bg-white/5"
                    onClick={() => setEditing(group)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        setEditing(group);
                      }
                    }}
                  >
                    <div className="min-w-0">
                      <div className="font-medium">{group.name}</div>
                      <div className="mt-1 text-sm text-[var(--color-muted)]">
                        {group.display_ids.length} display
                        {group.display_ids.length === 1 ? "" : "s"}
                        {group.hotkey && canActivate ? ` · ${group.hotkey}` : ""}
                        {!canActivate ? " · assign displays to enable" : ""}
                      </div>
                    </div>
                    <div
                      className="flex shrink-0 items-center gap-3"
                      onClick={(e) => e.stopPropagation()}
                      onKeyDown={(e) => e.stopPropagation()}
                    >
                      <Button
                        variant="secondary"
                        className="shrink-0"
                        disabled={!canActivate}
                        title={
                          canActivate
                            ? "Activate this group"
                            : "Add at least one display before activating"
                        }
                        onClick={() =>
                          api
                            .activateGroup(group.id)
                            .then((r) =>
                              toast.success(`Activated in ${r.display_apply_ms}ms`),
                            )
                            .catch((e) => toast.error(String(e)))
                        }
                      >
                        Activate
                      </Button>
                      <Button
                        variant="secondary"
                        size="icon"
                        className="shrink-0"
                        onClick={() => handleDeleteGroup(group.id)}
                        aria-label="Delete group"
                      >
                        <Trash2 size={16} className="text-rose-500" />
                      </Button>
                    </div>
                  </div>
                );
                })}
              </div>
            )}
          </div>
        ) : (
          <div className="space-y-3">
            <section className="form-section">
              <h2 className="text-base font-medium">Appearance</h2>
              <select
                className="form-select-control w-full"
                value={config.settings.theme}
                onChange={(e) => saveSettings({ theme: e.target.value })}
              >
                <option value="system">System</option>
                <option value="light">Light</option>
                <option value="dark">Dark</option>
              </select>
            </section>

            <section className="form-section">
              <h2 className="text-base font-medium">Startup</h2>
              <label className="form-check">
                <input
                  type="checkbox"
                  className="form-checkbox-control"
                  checked={config.settings.launch_on_startup}
                  onChange={(e) =>
                    saveSettings({ launch_on_startup: e.target.checked })
                  }
                />
                <span>Launch on Windows startup</span>
              </label>
              <label className="form-check">
                <input
                  type="checkbox"
                  className="form-checkbox-control"
                  checked={config.settings.minimize_to_tray}
                  onChange={(e) =>
                    saveSettings({ minimize_to_tray: e.target.checked })
                  }
                />
                <span>Start minimized to tray</span>
              </label>
              <p className="text-sm text-[var(--color-muted)]">
                The tray icon keeps hotkeys and gamepad chords active in the
                background. Minimize-to-tray applies on the next launch.
              </p>
            </section>

            <section className="form-section">
              <h2 className="text-base font-medium">Steam</h2>
              <input
                className="form-input-control w-full"
                placeholder="auto"
                value={config.settings.steam_path}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    settings: { ...config.settings, steam_path: e.target.value },
                  })
                }
                onBlur={() => saveSettings({ steam_path: config.settings.steam_path })}
              />
              <p className="text-sm text-[var(--color-muted)]">
                Leave as &quot;auto&quot; to detect from registry, or set a custom steam.exe path.
              </p>
            </section>

            <section className="form-section">
              <h2 className="text-base font-medium">Performance</h2>
              <TelemetryPanel />
            </section>
          </div>
        )}
        </div>
      </main>

      <Toaster richColors position="bottom-right" />
    </div>
  );
}

function TabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex h-8 cursor-pointer items-center gap-1.5 rounded-md px-3 text-sm transition-colors ${
        active
          ? "bg-[var(--color-accent)] text-white"
          : "text-[var(--color-muted)] hover:text-[var(--color-foreground)]"
      }`}
    >
      {children}
    </button>
  );
}

export default App;