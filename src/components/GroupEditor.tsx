import { useEffect, useState } from "react";
import { toast } from "sonner";
import { X, Save, Zap } from "lucide-react";
import { api, type DisplayGroup, type DisplayInfo } from "@/lib/api";
import { Button } from "@/components/ui/button";

interface Props {
  group: DisplayGroup;
  displays: DisplayInfo[];
  builtinActions: [string, string][];
  gamepadButtons: string[];
  onSave: (group: DisplayGroup) => Promise<void>;
  onClose: () => void;
}

export function GroupEditor({
  group,
  displays,
  builtinActions,
  gamepadButtons,
  onSave,
  onClose,
}: Props) {
  const [draft, setDraft] = useState(group);
  const [selectedDisplay, setSelectedDisplay] = useState("");
  const [recordingHotkey, setRecordingHotkey] = useState(false);
  useEffect(() => setDraft(group), [group]);

  const available = displays.filter((d) => !draft.display_ids.includes(d.id));

  const addDisplay = () => {
    if (!selectedDisplay) return;
    setDraft({ ...draft, display_ids: [...draft.display_ids, selectedDisplay] });
    setSelectedDisplay("");
  };

  const removeDisplay = (id: string) => {
    setDraft({
      ...draft,
      display_ids: draft.display_ids.filter((d) => d !== id),
    });
  };

  const handleSaveLayout = async () => {
    try {
      await onSave(draft);
      await api.saveGroupLayout(draft.id);
      toast.success("Layout saved");
    } catch (e) {
      toast.error(String(e));
    }
  };

  const handleActivate = async () => {
    try {
      const record = await api.activateGroup(draft.id, "ui");
      toast.success(`Activated in ${record.display_apply_ms}ms`);
    } catch (e) {
      toast.error(String(e));
    }
  };

  const handleSubmit = async () => {
    try {
      await onSave(draft);
      toast.success("Group saved");
      onClose();
    } catch (e) {
      toast.error(String(e));
    }
  };

  useEffect(() => {
    if (!recordingHotkey) return;
    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      const parts: string[] = [];
      if (e.ctrlKey) parts.push("Ctrl");
      if (e.altKey) parts.push("Alt");
      if (e.shiftKey) parts.push("Shift");
      if (e.metaKey) parts.push("Super");
      const key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
      if (!["Control", "Alt", "Shift", "Meta"].includes(e.key)) {
        parts.push(key);
        setDraft({ ...draft, hotkey: parts.join("+") });
        setRecordingHotkey(false);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [recordingHotkey, draft]);

  const toggleGamepadButton = (btn: string) => {
    const current = draft.gamepad_chord?.buttons ?? [];
    const next = current.includes(btn)
      ? current.filter((b) => b !== btn)
      : [...current, btn];
    setDraft({
      ...draft,
      gamepad_chord: next.length
        ? { buttons: next, hold_ms: draft.gamepad_chord?.hold_ms ?? 400 }
        : null,
    });
  };

  const launchAction =
    draft.post_action.type === "launch-program" ? draft.post_action : null;
  const runAction =
    draft.post_action.type === "run-command" ? draft.post_action : null;

  return (
    <div className="space-y-6">
      <div>
        <label className="text-sm text-[var(--color-muted)]">Group name</label>
        <input
          className="mt-1 w-full rounded-lg border border-[var(--color-card-border)] bg-[var(--color-card)] px-3 py-2"
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        />
      </div>

      <div>
        <label className="text-sm text-[var(--color-muted)]">Displays</label>
        <div className="mt-2 flex gap-2">
          <select
            className="flex-1 rounded-lg border border-[var(--color-card-border)] bg-[var(--color-card)] px-3 py-2"
            value={selectedDisplay}
            onChange={(e) => setSelectedDisplay(e.target.value)}
          >
            <option value="">Select a display…</option>
            {available.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name} ({d.width}×{d.height})
              </option>
            ))}
          </select>
          <Button variant="secondary" onClick={addDisplay}>
            Add
          </Button>
        </div>
        <ul className="mt-3 space-y-2">
          {draft.display_ids.map((id) => {
            const display = displays.find((d) => d.id === id);
            return (
              <li
                key={id}
                className="flex items-center justify-between rounded-lg border border-[var(--color-card-border)] px-3 py-2"
              >
                <span>{display?.name ?? id}</span>
                <button
                  onClick={() => removeDisplay(id)}
                  className="text-[var(--color-muted)] hover:text-rose-500"
                >
                  <X size={16} />
                </button>
              </li>
            );
          })}
        </ul>
      </div>

      <div className="flex flex-wrap gap-2">
        <Button variant="secondary" onClick={handleSaveLayout}>
          <Save size={16} /> Save layout
        </Button>
        <Button onClick={handleActivate}>
          <Zap size={16} /> Activate now
        </Button>
      </div>

      <div>
        <label className="text-sm text-[var(--color-muted)]">Hotkey</label>
        <div className="mt-2 flex gap-2">
          <input
            readOnly
            className="flex-1 rounded-lg border border-[var(--color-card-border)] bg-[var(--color-card)] px-3 py-2"
            value={draft.hotkey ?? "None"}
            placeholder="None"
          />
          <Button
            variant="secondary"
            onClick={() => setRecordingHotkey(true)}
          >
            {recordingHotkey ? "Press keys…" : "Record"}
          </Button>
          <Button
            variant="ghost"
            onClick={() => setDraft({ ...draft, hotkey: null })}
          >
            Clear
          </Button>
        </div>
      </div>

      <div>
        <label className="text-sm text-[var(--color-muted)]">
          Gamepad chord (hold {draft.gamepad_chord?.hold_ms ?? 400}ms)
        </label>
        <div className="mt-2 flex flex-wrap gap-2">
          {gamepadButtons.map((btn) => {
            const active = draft.gamepad_chord?.buttons.includes(btn);
            return (
              <Button
                key={btn}
                size="sm"
                variant={active ? "default" : "secondary"}
                onClick={() => toggleGamepadButton(btn)}
              >
                {btn}
              </Button>
            );
          })}
        </div>
      </div>

      <div>
        <label className="text-sm text-[var(--color-muted)]">Post-action</label>
        <select
          className="mt-1 w-full rounded-lg border border-[var(--color-card-border)] bg-[var(--color-card)] px-3 py-2"
          value={
            draft.post_action.type === "builtin"
              ? draft.post_action.action
              : draft.post_action.type
          }
          onChange={(e) => {
            const val = e.target.value;
            if (val === "launch-program" || val === "run-command") {
              setDraft({
                ...draft,
                post_action:
                  val === "launch-program"
                    ? { type: "launch-program", path: "", args: "" }
                    : { type: "run-command", command: "" },
              });
            } else {
              setDraft({
                ...draft,
                post_action: { type: "builtin", action: val },
              });
            }
          }}
        >
          {builtinActions.map(([id, label]) => (
            <option key={id} value={id}>
              {label}
            </option>
          ))}
          <option value="launch-program">Launch a program</option>
          <option value="run-command">Run shell command</option>
        </select>

        {launchAction && (
          <div className="mt-2 space-y-2">
            <input
              className="w-full rounded-lg border border-[var(--color-card-border)] bg-[var(--color-card)] px-3 py-2"
              placeholder="C:\path\to\program.exe"
              value={launchAction.path}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  post_action: {
                    type: "launch-program",
                    path: e.target.value,
                    args: launchAction.args,
                  },
                })
              }
            />
            <input
              className="w-full rounded-lg border border-[var(--color-card-border)] bg-[var(--color-card)] px-3 py-2"
              placeholder="Optional arguments"
              value={launchAction.args ?? ""}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  post_action: {
                    type: "launch-program",
                    path: launchAction.path,
                    args: e.target.value,
                  },
                })
              }
            />
          </div>
        )}

        {runAction && (
          <input
            className="mt-2 w-full rounded-lg border border-[var(--color-card-border)] bg-[var(--color-card)] px-3 py-2"
            placeholder="command to run"
            value={runAction.command}
            onChange={(e) =>
              setDraft({
                ...draft,
                post_action: {
                  type: "run-command",
                  command: e.target.value,
                },
              })
            }
          />
        )}
      </div>

      <div className="flex justify-end gap-2 border-t border-[var(--color-card-border)] pt-4">
        <Button variant="secondary" onClick={onClose}>
          Cancel
        </Button>
        <Button onClick={handleSubmit}>Save group</Button>
      </div>
    </div>
  );
}