import { useState } from "react";
import { Monitor, Keyboard, CheckCircle2 } from "lucide-react";
import { api, formatDisplayStatus, type DisplayInfo } from "@/lib/api";
import { Button } from "@/components/ui/button";

interface Props {
  displays: DisplayInfo[];
  onComplete: () => void;
}

export function OnboardingWizard({ displays, onComplete }: Props) {
  const [step, setStep] = useState(0);
  const [creating, setCreating] = useState(false);

  const steps = [
    {
      title: "Welcome to Display Switcher",
      body: "Create display groups, bind hotkeys or gamepad chords, and switch layouts instantly.",
      icon: Monitor,
    },
    {
      title: "Your displays",
      body: `Found ${displays.length} connected display${displays.length === 1 ? "" : "s"}. You'll assign them to groups in the next step.`,
      icon: Monitor,
    },
    {
      title: "Quick setup",
      body: "We'll create Desktop Mode and TV Mode groups to get you started. You can customize them anytime.",
      icon: Keyboard,
    },
  ];

  const StepIcon = steps[step].icon;

  const finish = async () => {
    setCreating(true);
    try {
      const desktop = await api.createGroup("Desktop Mode");
      const tv = await api.createGroup("TV Mode");

      await api.updateGroup({
        ...desktop,
        hotkey: "Ctrl+Alt+D",
        post_action: { type: "builtin", action: "exit-steam-big-picture" },
      });
      await api.updateGroup({
        ...tv,
        hotkey: "Ctrl+Alt+T",
        post_action: { type: "builtin", action: "launch-steam-big-picture" },
      });

      await api.completeOnboarding();
      onComplete();
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="mx-auto flex min-h-[80vh] max-w-lg flex-col items-center justify-center px-3 text-center">
      <div className="mb-3 flex h-14 w-14 items-center justify-center rounded-2xl bg-[var(--color-accent)] text-white">
        <StepIcon size={28} />
      </div>
      <h1 className="text-2xl font-semibold">{steps[step].title}</h1>
      <p className="mt-3 text-[var(--color-muted)]">{steps[step].body}</p>

      {step === 1 && (
        <ul className="mt-3 w-full space-y-3 text-left">
          {displays.map((d) => (
            <li
              key={d.id}
              className="rounded-lg border border-[var(--color-card-border)] p-3"
            >
              <div className="font-medium">{d.name}</div>
              <div className="text-sm text-[var(--color-muted)]">
                {formatDisplayStatus(d)}
                {d.is_primary ? " · Primary" : ""}
              </div>
            </li>
          ))}
        </ul>
      )}

      <div className="mt-3 flex gap-3">
        {step > 0 && (
          <Button variant="secondary" onClick={() => setStep(step - 1)}>
            Back
          </Button>
        )}
        {step < steps.length - 1 ? (
          <Button onClick={() => setStep(step + 1)}>Continue</Button>
        ) : (
          <Button onClick={finish} disabled={creating}>
            <CheckCircle2 size={16} />
            {creating ? "Creating…" : "Get started"}
          </Button>
        )}
      </div>
    </div>
  );
}