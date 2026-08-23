import createClient, { type Client } from "openapi-fetch";

import type { paths } from "./schema";

/**
 * Base URL the API is reached at.
 *
 * Server-side code talks to the API directly over `API_URL`, which may be an
 * internal address the browser cannot resolve. Browser code needs a publicly
 * reachable origin, so it uses `NEXT_PUBLIC_API_URL` — inlined at build time —
 * and relies on the API's CORS allow-list, which already covers the web app.
 */
function resolveBaseUrl(): string {
  const url =
    typeof window === "undefined"
      ? (process.env.API_URL ?? process.env.NEXT_PUBLIC_API_URL)
      : process.env.NEXT_PUBLIC_API_URL;

  return (url ?? "http://localhost:8080").replace(/\/+$/, "");
}

/**
 * Creates a client bound to `paths`, so route strings, request bodies and
 * responses are all checked against the generated OpenAPI types.
 *
 * Prefer the shared `api` export; this exists for tests and for the rare call
 * that needs to target a different host.
 */
export function createApiClient(baseUrl = resolveBaseUrl()): Client<paths> {
  return createClient<paths>({
    baseUrl,
    // Next.js caches `fetch` responses by default. API data is request-scoped,
    // so opt out here and let individual calls opt back in via `next`/`cache`.
    cache: "no-store",
  });
}

/** Shared client. Safe to import from Server Components, Route Handlers and Client Components. */
export const api = createApiClient();

/** Authorization header for an endpoint marked with `bearer_auth`. */
export function bearer(accessToken: string) {
  return { Authorization: `Bearer ${accessToken}` } as const;
}
