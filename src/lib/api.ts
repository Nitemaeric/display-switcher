import { invoke } from "@tauri-apps/api/core";

export interface DisplayInfo {
  id: string;
  name: string;
  is_active: boolean;
  is_primary: boolean;
  width: number;
  height: number;
  x: number;
  y: number;
}

export function formatDisplayStatus(display: DisplayInfo): string {
  if (!display.is_active) {
    return "Disabled in Windows";
  }
  if (display.width > 0 && display.height > 0) {
    return `${display.width}×${display.height}`;
  }
  return "Connected";
}

export function formatInvokeError(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  if (error && typeof error === "object") {
    const record = error as Record<string, unknown>;
    if (typeof record.message === "string") {
      return record.message;
    }
  }
  return String(error);
}

export function formatAssignedDisplayNames(
  displayIds: string[],
  displays: DisplayInfo[],
): string {
  if (displayIds.length === 0) {
    return "No displays assigned";
  }

  const names = displayIds.map((id) => {
    const display = displays.find((d) => d.id === id);
    return display?.name ?? id;
  });

  return names.join(", ");
}

export function formatGroupSummary(
  displayIds: string[],
  displays: DisplayInfo[],
  hasLayout: boolean,
  hotkey?: string | null,
): string {
  const assigned = formatAssignedDisplayNames(displayIds, displays);

  if (displayIds.length === 0) {
    return assigned;
  }

  if (!hasLayout) {
    return `${assigned} assigned · capture layout to activate`;
  }

  return hotkey ? `${assigned} · ${hotkey}` : assigned;
}

export function formatDisplayLabel(display: DisplayInfo): string {
  const status = formatDisplayStatus(display);
  if (status === "Connected") {
    return display.name;
  }
  if (status === "Disabled in Windows") {
    return `${display.name} (disabled)`;
  }
  return `${display.name} (${status})`;
}

export interface GamepadChord {
  buttons: string[];
  hold_ms: number;
}

export type PostAction =
  | { type: "builtin"; action: string }
  | { type: "launch-program"; path: string; args?: string }
  | { type: "run-command"; command: string };

export interface DisplayGroup {
  id: string;
  name: string;
  display_ids: string[];
  profile_file: string;
  hotkey?: string | null;
  gamepad_chord?: GamepadChord | null;
  post_action: PostAction;
}

export interface AppSettings {
  theme: string;
  launch_on_startup: boolean;
  steam_path: string;
  minimize_to_tray: boolean;
  telemetry_retention: number;
}

export interface AppConfig {
  version: number;
  settings: AppSettings;
  groups: DisplayGroup[];
  onboarding_complete: boolean;
}

export interface SwitchRecord {
  timestamp: string;
  group_id: string;
  group_name: string;
  trigger: string;
  display_apply_ms: number;
  post_action_ms: number;
  total_ms: number;
  success: boolean;
  error?: string;
}

export interface TelemetryStats {
  count: number;
  median_display_apply_ms: number;
  p95_display_apply_ms: number;
  success_rate: number;
}

export const api = {
  getConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<void>("save_app_config", { config }),
  listDisplays: () => invoke<DisplayInfo[]>("list_displays"),
  listGroupLayoutStatus: () =>
    invoke<Record<string, boolean>>("list_group_layout_status"),
  saveGroupLayout: (groupId: string) =>
    invoke<void>("save_group_layout_cmd", { groupId }),
  activateGroup: (groupId: string, trigger = "ui") =>
    invoke<SwitchRecord>("activate_group_cmd", { groupId, trigger }),
  createGroup: (name: string) => invoke<DisplayGroup>("create_group", { name }),
  deleteGroup: (groupId: string) => invoke<void>("delete_group", { groupId }),
  updateGroup: (group: DisplayGroup) => invoke<void>("update_group", { group }),
  getBuiltinActions: () =>
    invoke<[string, string][]>("get_builtin_actions"),
  getGamepadButtons: () => invoke<string[]>("get_gamepad_buttons"),
  getTelemetryStats: () => invoke<TelemetryStats>("get_telemetry_stats"),
  getTelemetryRecent: (limit: number) =>
    invoke<SwitchRecord[]>("get_telemetry_recent", { limit }),
  clearTelemetry: () => invoke<void>("clear_telemetry"),
  exportTelemetry: (path: string) => invoke<void>("export_telemetry", { path }),
  completeOnboarding: () => invoke<void>("complete_onboarding"),
  syncWindowChrome: (theme: string) =>
    invoke<void>("sync_window_chrome", { theme }),
  resolveThemeSetting: (theme: string) =>
    invoke<string>("resolve_theme_setting", { theme }),
};