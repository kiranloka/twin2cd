import { NextRequest, NextResponse } from "next/server";

async function proxy(request: NextRequest, context: { params: Promise<{ path: string[] }> }) {
  const base = process.env.FORGE_API_URL;
  const token = process.env.FORGE_API_TOKEN;
  if (!base || !token) return NextResponse.json({ error: "Forge API is not configured" }, { status: 503 });
  const { path } = await context.params;
  const upstream = new URL(`/api/v1/${path.map(encodeURIComponent).join("/")}`, base);
  upstream.search = request.nextUrl.search;
  try {
    const response = await fetch(upstream, {
      method: "GET",
      headers: { authorization: `Bearer ${token}` },
      cache: "no-store",
    });
    return new NextResponse(response.body, { status: response.status, headers: { "content-type": response.headers.get("content-type") ?? "application/json" } });
  } catch {
    return NextResponse.json({ error: "Forge API is unavailable" }, { status: 502 });
  }
}

export const GET = proxy;
