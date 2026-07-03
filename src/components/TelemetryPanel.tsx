import { useEffect, useState } from "react";
import { api, type SwitchRecord, type TelemetryStats } from "@/lib/api";
import { Button } from "@/components/ui/button";

export function TelemetryPanel() {
  const [stats, setStats] = useState<TelemetryStats | null>(null);
  const [recent, setRecent] = useState<SwitchRecord[]>([]);

  const load = async () => {
    setStats(await api.getTelemetryStats());
    setRecent(await api.getTelemetryRecent(10));
  };

  useEffect(() => {
    load();
  }, []);

  return (
    <div className="w-full space-y-3">
      {stats && (
        <div className="grid w-full grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label="Samples" value={String(stats.count)} />
          <Stat
            label="Median apply"
            value={`${stats.median_display_apply_ms}ms`}
          />
          <Stat label="P95 apply" value={`${stats.p95_display_apply_ms}ms`} />
          <Stat
            label="Success rate"
            value={`${Math.round(stats.success_rate * 100)}%`}
          />
        </div>
      )}

      <div className="w-full overflow-hidden rounded-lg border border-[var(--color-card-border)]">
        <table className="w-full text-sm">
          <thead className="bg-black/5 dark:bg-white/5">
            <tr>
              <th className="px-3 py-3 text-left font-medium">Group</th>
              <th className="px-3 py-3 text-left font-medium">Trigger</th>
              <th className="px-3 py-3 text-right font-medium">Apply</th>
              <th className="px-3 py-3 text-right font-medium">Total</th>
            </tr>
          </thead>
          <tbody>
            {recent.length === 0 ? (
              <tr>
                <td colSpan={4} className="px-3 py-3 text-center text-[var(--color-muted)]">
                  No switches recorded yet
                </td>
              </tr>
            ) : (
              recent.map((r) => (
                <tr
                  key={r.timestamp + r.group_id}
                  className="border-t border-[var(--color-card-border)]"
                >
                  <td className="px-3 py-3">{r.group_name}</td>
                  <td className="px-3 py-3 text-[var(--color-muted)]">
                    {r.trigger}
                  </td>
                  <td className="px-3 py-3 text-right">{r.display_apply_ms}ms</td>
                  <td className="px-3 py-3 text-right">{r.total_ms}ms</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      <Button
        variant="secondary"
        onClick={async () => {
          await api.clearTelemetry();
          await load();
        }}
      >
        Clear telemetry
      </Button>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-[var(--color-card-border)] p-3">
      <div className="text-xs text-[var(--color-muted)]">{label}</div>
      <div className="mt-3 text-lg font-semibold">{value}</div>
    </div>
  );
}