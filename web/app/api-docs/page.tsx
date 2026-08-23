import type { Metadata } from "next";

import SwaggerDocs from "./swagger";

export const metadata: Metadata = {
  title: "API reference",
  description: "OpenAPI reference for the Taxonomy Follower API.",
};

export default function ApiDocsPage() {
  // Served by app/api/openapi/route.ts, which proxies the Rust API.
  return (
    <div className="flex-1 bg-white text-black">
      <SwaggerDocs url="/api/openapi" />
    </div>
  );
}
