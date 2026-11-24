"use client";

import { useMemo, useState } from "react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Textarea } from "@/components/ui/textarea";
import type { ValidateWorkflowResponse } from "@forge/types";

const initialYaml = `version: "1"
name: onepos-production
jobs:
  test:
    name: Test
    steps:
      - name: Run tests
        run: pnpm test
  build:
    name: Build
    needs: [test]
    steps:
      - name: Build app
        run: pnpm build
  deploy:
    name: Deploy
    needs: [build]
    steps:
      - name: Deploy production
        run: ./deploy.sh
`;

type JobGraph = { id: string; needs: string[] };

// The server compiler remains the authority. This extracts dependency labels
// from the conventional Forgefile shape for a readable inline graph preview.
function readJobGraph(yaml: string): JobGraph[] {
  const lines = yaml.split(/\r?\n/);
  const jobsLine = lines.findIndex((line) => /^jobs:\s*(#.*)?$/.test(line));
  if (jobsLine < 0) return [];

  const jobs: JobGraph[] = [];
  let current: JobGraph | undefined;
  let inNeedsList = false;
  for (const line of lines.slice(jobsLine + 1)) {
    const jobMatch = line.match(/^ {2}([\w.-]+):\s*(?:#.*)?$/);
    if (jobMatch) {
      current = { id: jobMatch[1], needs: [] };
      jobs.push(current);
      inNeedsList = false;
      continue;
    }
    if (!current) continue;
    const needsMatch = line.match(/^ {4}needs:\s*(.*)$/);
    if (needsMatch) {
      const inline = needsMatch[1].match(/^\[([^\]]*)\]/)?.[1];
      if (inline) {
        current.needs.push(...inline.split(",").map((item) => item.trim().replace(/^['"]|['"]$/g, "")).filter(Boolean));
        inNeedsList = false;
      } else {
        inNeedsList = true;
      }
      continue;
    }
    const listItem = line.match(/^ {6}-\s*([\w.-]+)/);
    if (inNeedsList && listItem) {
      current.needs.push(listItem[1]);
      continue;
    }
    if (/^ {4}\S/.test(line)) inNeedsList = false;
  }
  return jobs;
}

export function WorkflowEditor() {
  const [yaml, setYaml] = useState(initialYaml);
  const [result, setResult] = useState<ValidateWorkflowResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [requestError, setRequestError] = useState<string | null>(null);
  const graph = useMemo(() => readJobGraph(yaml), [yaml]);

  async function validate() {
    setLoading(true);
    setRequestError(null);
    setResult(null);
    try {
      const response = await fetch("/api/workflows/validate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ yaml }),
      });
      if (!response.ok) throw new Error(`Validation request failed (${response.status})`);
      setResult((await response.json()) as ValidateWorkflowResponse);
    } catch (error) {
      setRequestError(error instanceof Error ? error.message : "Could not reach the Forge API");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="grid gap-6 xl:grid-cols-[minmax(0,1.15fr)_minmax(20rem,0.85fr)]">
      <Card>
        <CardHeader>
          <CardTitle>Forgefile.yaml</CardTitle>
          <CardDescription>Edit the workflow definition, then validate it with the controller.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <Textarea
            aria-label="Forgefile YAML"
            spellCheck={false}
            value={yaml}
            onChange={(event) => {
              setYaml(event.target.value);
              setResult(null);
              setRequestError(null);
            }}
            className="min-h-[28rem] resize-y rounded-lg bg-muted/40 font-mono text-xs leading-6"
          />
          <div className="flex flex-wrap items-center gap-3">
            <Button onClick={validate} disabled={loading}>
              {loading ? "Validating…" : "Validate workflow"}
            </Button>
            <span className="text-xs text-muted-foreground">Validation runs through the Rust workflow compiler.</span>
          </div>
          {result && (
            <Alert variant={result.valid ? "default" : "destructive"}>
              <AlertTitle>{result.valid ? "Workflow is valid" : "Workflow has errors"}</AlertTitle>
              <AlertDescription>
                {result.valid ? `${result.job_order.length} jobs compiled successfully.` : result.error}
              </AlertDescription>
            </Alert>
          )}
          {requestError && (
            <Alert variant="destructive">
              <AlertTitle>Could not validate</AlertTitle>
              <AlertDescription>{requestError}. Check that the Forge API is running and reachable.</AlertDescription>
            </Alert>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between gap-3">
            <CardTitle>Job graph</CardTitle>
            <Badge variant={result?.valid ? "default" : "secondary"}>{result?.valid ? "Validated" : "Live preview"}</Badge>
          </div>
          <CardDescription>Dependencies from the current YAML, arranged in compiler execution order.</CardDescription>
        </CardHeader>
        <CardContent>
          {graph.length ? (
            <ol className="space-y-3">
              {(result?.valid ? result.job_order.map((id) => graph.find((job) => job.id === id)).filter((job): job is JobGraph => Boolean(job)) : graph).map((job, index) => (
                <li key={job.id} className="relative rounded-lg border bg-background p-3">
                  {index > 0 && <div aria-hidden="true" className="absolute -top-3 left-5 h-3 border-l border-border" />}
                  <div className="flex items-center gap-2">
                    <span className="grid size-6 place-items-center rounded-full bg-muted text-xs text-muted-foreground">{index + 1}</span>
                    <span className="font-medium">{job.id}</span>
                  </div>
                  <p className="mt-2 pl-8 text-xs text-muted-foreground">
                    {job.needs.length ? `Needs ${job.needs.join(", ")}` : "No dependencies · ready to run"}
                  </p>
                </li>
              ))}
            </ol>
          ) : (
            <p className="rounded-lg border border-dashed p-6 text-sm text-muted-foreground">Add jobs under <code>jobs:</code> to see the graph preview.</p>
          )}
          {result?.valid && result.job_order.length !== graph.length && (
            <p className="mt-3 text-xs text-muted-foreground">The preview only includes jobs recognized in the current editor format.</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
