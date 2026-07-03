import { useEffect, useState } from "react";
import { toast } from "sonner";
import { X, Save, Zap } from "lucide-react";
import { api, type DisplayGroup, type DisplayInfo } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

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
  const canActivate = draft.display_ids.length > 0;

  return (
    <div className="form-section">
      <div>
        <label className="form-label">Group name</label>
        <input
          className={cn("form-input-control mt-3 w-full")}
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        />
      </div>

      <div>
        <label className="form-label">Displays</label>
        <div className="form-row">
          <select
            className="form-select-control min-w-0 flex-1"
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
          <Button variant="secondary" className="shrink-0" onClick={addDisplay}>
            Add
          </Button>
        </div>
        <ul className="mt-3 space-y-3">
          {draft.display_ids.map((id) => {
            const display = displays.find((d) => d.id === id);
            return (
              <li
                key={id}
                className="flex h-10 items-center justify-between rounded-lg border border-[var(--color-card-border)] px-3"
              >
                <span className="text-sm">{display?.name ?? id}</span>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 shrink-0"
                  onClick={() => removeDisplay(id)}
                  aria-label="Remove display"
                >
                  <X size={16} />
                </Button>
              </li>
            );
          })}
        </ul>
      </div>

      <div className="form-actions">
        <Button variant="secondary" className="shrink-0" onClick={handleSaveLayout}>
          <Save size={16} /> Save layout
        </Button>
        <Button
          className="shrink-0"
          onClick={handleActivate}
          disabled={!canActivate}
          title={
            canActivate
              ? "Activate this group"
              : "Add at least one display before activating"
          }
        >
          <Zap size={16} /> Activate now
        </Button>
      </div>

      <div>
        <label className="form-label">Hotkey</label>
        <div className="form-row">
          <input
            readOnly
            className="form-input-control min-w-0 flex-1"
            value={draft.hotkey ?? "None"}
            placeholder="None"
          />
          <Button
            variant="secondary"
            className="shrink-0"
            onClick={() => setRecordingHotkey(true)}
          >
            {recordingHotkey ? "Press keys…" : "Record"}
          </Button>
          <Button
            variant="secondary"
            className="shrink-0"
            onClick={() => setDraft({ ...draft, hotkey: null })}
          >
            Clear
          </Button>
        </div>
      </div>

      <div>
        <label className="form-label">
          Gamepad chord (hold {draft.gamepad_chord?.hold_ms ?? 400}ms)
        </label>
        <div className="form-actions">
          {gamepadButtons.map((btn) => {
            const active = draft.gamepad_chord?.buttons.includes(btn);
            return (
              <Button
                key={btn}
                variant={active ? "default" : "secondary"}
                className="min-w-[3.25rem] shrink-0 px-3"
                onClick={() => toggleGamepadButton(btn)}
              >
                {btn}
              </Button>
            );
          })}
        </div>
      </div>

      <div>
        <label className="form-label">Post-action</label>
        <select
          className={cn("form-select-control mt-3 w-full")}
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
          <div className="form-section mt-3">
            <input
              className="form-input-control w-full"
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
              className="form-input-control w-full"
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
            className="form-input-control mt-3 w-full"
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

      <div className="flex items-center justify-end gap-3 border-t border-[var(--color-card-border)] pt-3">
        <Button
          variant="secondary"
          className="min-w-[7rem]"
          onClick={onClose}
        >
          Cancel
        </Button>
        <Button className="min-w-[7rem]" onClick={handleSubmit}>
          Save group
        </Button>
      </div>
    </div>
  );
}