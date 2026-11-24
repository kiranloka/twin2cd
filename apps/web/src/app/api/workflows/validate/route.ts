import { NextResponse } from "next/server";

export async function POST(request: Request) {
  let body: { yaml?: unknown };
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ valid: false, job_order: [], error: "Request body must be valid JSON" }, { status: 400 });
  }

  if (typeof body.yaml !== "string") {
    return NextResponse.json({ valid: false, job_order: [], error: "yaml must be a string" }, { status: 400 });
  }

  try {
    const response = await fetch(`${process.env.FORGE_API_URL ?? "http://localhost:8080"}/api/v1/workflows/validate`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ yaml: body.yaml }),
      cache: "no-store",
    });
    const result = await response.json();
    return NextResponse.json(result, { status: response.status });
  } catch {
    return NextResponse.json({ valid: false, job_order: [], error: "Forge API is unavailable" }, { status: 503 });
  }
}
