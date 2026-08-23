/**
 * Restores the ApiDOM element registration that Turbopack tree-shakes away.
 *
 * `@swagger-api/apidom-core` and `@swagger-api/apidom-ns-openapi-3-1` each ship
 * a `src/refractor/registration.mjs` that exists only for its side effect: it
 * assigns a `refract` static onto every element class. Both packages list those
 * files in their `sideEffects` field, but Turbopack collapses the re-export
 * chain (`index.mjs` -> `registration.mjs` -> `minim`) and binds consumers
 * straight to the source module, so the registration file is never included in
 * the graph and the statics are never assigned.
 *
 * Swagger UI then throws `OpenApi3_1Element.refract is not a function` while
 * resolving the document. The operation list and the schema list still render,
 * which makes it look like it worked, but every operation body comes out empty:
 * no parameters, no responses, no "Try it out".
 *
 * Importing the two files directly forces them into the graph. They have to be
 * addressed by path because neither package exposes a subpath in its `exports`
 * map — which is also why both are direct dependencies of this app, pinned to
 * the same `^1.12.0` range `swagger-client` asks for so pnpm keeps resolving
 * them to a single copy. Import this module before anything loads Swagger UI.
 *
 * Drop this file once Turbopack honours the `sideEffects` field here.
 */
import "../../node_modules/@swagger-api/apidom-core/src/refractor/registration.mjs";
import "../../node_modules/@swagger-api/apidom-ns-openapi-3-1/src/refractor/registration.mjs";
