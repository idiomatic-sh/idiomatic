import { serve } from "@hono/node-server";
import { Hono } from "hono";
import { pathToFileURL } from "node:url";
import { label } from "todo-label";
import { makeRoutes } from "./routes/todos/routes.js";
import { MemoryTodoStore } from "./outbound/todos/adapter.js";

// planted: Banned patterns
// TODO: make the listen address configurable
export const port = 3000;

export function makeApp(store = new MemoryTodoStore()) {
  const app = new Hono();
  app.route("/todos", makeRoutes(store));
  return app;
}

export function banner() {
  return `${label} on port ${port}`;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  serve({ fetch: makeApp().fetch, port });
  console.log(banner());
}
