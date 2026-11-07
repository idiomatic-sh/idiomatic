import { serve } from "@hono/node-server";
import { Hono } from "hono";
import { routes } from "./routes/todos/routes";

// TODO: read the port from the environment (planted: Banned patterns)
const port = 3000;

export function banner(label: any): string {
  return `todo api ${label}`;
}

const app = new Hono();
app.route("/todos", routes);

if (import.meta.url === `file://${process.argv[1]}`) {
  serve({ fetch: app.fetch, port });
  console.log(banner(`on port ${port}`));
}

export default app;
