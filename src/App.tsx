import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Monitor, Plus, Settings, Trash2, Pencil } from "lucide-react";
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
      <div className="min-h-screen p-6">
        <div className="mx-auto max-w-2xl">
          <h2 className="mb-6 text-xl font-semibold">Edit group</h2>
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
      <header className="border-b border-[var(--color-card-border)] px-6 py-4">
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
          <nav className="flex gap-1 rounded-lg border border-[var(--color-card-border)] p-1">
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

      <main className="mx-auto max-w-4xl px-6 py-6">
        {tab === "groups" ? (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-base font-medium">Display groups</h2>
              <Button size="sm" onClick={handleCreateGroup}>
                <Plus size={16} /> Add group
              </Button>
            </div>

            {config.groups.length === 0 ? (
              <div className="rounded-xl border border-dashed border-[var(--color-card-border)] py-12 text-center text-[var(--color-muted)]">
                No groups yet. Create one to get started.
              </div>
            ) : (
              <div className="grid gap-3">
                {config.groups.map((group) => (
                  <div
                    key={group.id}
                    className="flex items-center justify-between rounded-xl border border-[var(--color-card-border)] bg-[var(--color-card)] p-4"
                  >
                    <div>
                      <div className="font-medium">{group.name}</div>
                      <div className="mt-1 text-sm text-[var(--color-muted)]">
                        {group.display_ids.length} display
                        {group.display_ids.length === 1 ? "" : "s"}
                        {group.hotkey ? ` · ${group.hotkey}` : ""}
                      </div>
                    </div>
                    <div className="flex gap-2">
                      <Button
                        size="sm"
                        variant="secondary"
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
                        size="sm"
                        variant="ghost"
                        onClick={() => setEditing(group)}
                      >
                        <Pencil size={16} />
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => handleDeleteGroup(group.id)}
                      >
                        <Trash2 size={16} className="text-rose-500" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        ) : (
          <div className="space-y-8">
            <section className="space-y-4">
              <h2 className="text-base font-medium">Appearance</h2>
              <select
                className="rounded-lg border border-[var(--color-card-border)] bg-[var(--color-card)] px-3 py-2"
                value={config.settings.theme}
                onChange={(e) => saveSettings({ theme: e.target.value })}
              >
                <option value="system">System</option>
                <option value="light">Light</option>
                <option value="dark">Dark</option>
              </select>
            </section>

            <section className="space-y-4">
              <h2 className="text-base font-medium">Steam</h2>
              <input
                className="w-full rounded-lg border border-[var(--color-card-border)] bg-[var(--color-card)] px-3 py-2"
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

            <section className="space-y-4">
              <h2 className="text-base font-medium">Performance</h2>
              <TelemetryPanel />
            </section>
          </div>
        )}
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
      className={`flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm transition-colors ${
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