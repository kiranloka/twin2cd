"use client";

import { useEffect, useState } from "react";
type ForgeLog = { log_id: string; stream: string; line: string; created_at: number };

export function LiveLogs({ runId }: { runId: string }) {
  const [logs, setLogs] = useState<ForgeLog[]>([]);
  const [error, setError] = useState(false);

  useEffect(() => {
    let active = true;
    let after: string | undefined;
    const poll = async () => {
      try {
        const query = after ? `?after=${encodeURIComponent(after)}` : "";
        const response = await fetch(`/api/forge/runs/${encodeURIComponent(runId)}/logs${query}`, { cache: "no-store" });
        if (!response.ok) throw new Error("log request failed");
        const rows = (await response.json()) as ForgeLog[];
        if (!active) return;
        if (rows.length) {
          after = rows[rows.length - 1].log_id;
          setLogs((current) => [...current, ...rows].slice(-5000));
        }
        setError(false);
      } catch {
        if (active) setError(true);
      }
    };
    void poll();
    const timer = window.setInterval(poll, 1500);
    return () => { active = false; window.clearInterval(timer); };
  }, [runId]);

  return <section className="rounded-lg border bg-muted/30 p-4">
    <h2 className="mb-3 font-semibold">Live logs</h2>
    {error && <p className="mb-2 text-sm text-destructive">Log stream is temporarily unavailable.</p>}
    <pre className="max-h-[60vh] overflow-auto whitespace-pre-wrap font-mono text-xs leading-5">
      {logs.length ? logs.map((log) => <span key={log.log_id} className={log.stream === "stderr" ? "text-destructive" : ""}>[{log.stream}] {log.line}{"\n"}</span>) : "Waiting for runner output…"}
    </pre>
  </section>;
}
