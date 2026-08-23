// `swagger-ui-react` ships no type declarations and the DefinitelyTyped package
// pins @types/react 18, which conflicts with React 19. This covers the props we
// actually pass.
declare module "swagger-ui-react" {
  import type { ComponentType } from "react";

  export interface SwaggerUIProps {
    /** URL the OpenAPI document is fetched from. */
    url?: string;
    /** Already-parsed OpenAPI document, as an alternative to `url`. */
    spec?: object;
    docExpansion?: "list" | "full" | "none";
    defaultModelsExpandDepth?: number;
    defaultModelExpandDepth?: number;
    displayRequestDuration?: boolean;
    filter?: boolean | string;
    tryItOutEnabled?: boolean;
    persistAuthorization?: boolean;
    supportedSubmitMethods?: Array<
      "get" | "put" | "post" | "delete" | "options" | "head" | "patch" | "trace"
    >;
    requestInterceptor?: (request: unknown) => unknown;
    responseInterceptor?: (response: unknown) => unknown;
    onComplete?: (system: unknown) => void;
  }

  const SwaggerUI: ComponentType<SwaggerUIProps>;
  export default SwaggerUI;
}

declare module "swagger-ui-react/swagger-ui.css";
