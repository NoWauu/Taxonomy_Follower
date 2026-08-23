"use client";

import dynamic from "next/dynamic";

// Must come before Swagger UI is pulled in. See the file for why it exists.
import "./apidom-registration";
import "swagger-ui-react/swagger-ui.css";

// swagger-ui-react reaches for browser globals while rendering, so it must be
// kept out of the server render. `ssr: false` is only allowed inside a Client
// Component, which is why this wrapper exists.
const SwaggerUI = dynamic(() => import("swagger-ui-react"), {
  ssr: false,
  loading: () => <p className="p-8 text-zinc-600">Loading API reference…</p>,
});

export default function SwaggerDocs({ url }: { url: string }) {
  return (
    <SwaggerUI
      url={url}
      docExpansion="list"
      defaultModelsExpandDepth={1}
      displayRequestDuration
      persistAuthorization
      filter
    />
  );
}
