# Todo API — JavaScript

Hono on plain ESM JavaScript and Node 22+. Each app owns an in-memory store.
Routes translate HTTP; use cases take the store port; todo types and title logic live beside that port.

## Install

`npm install` installs the registry dependencies and the local `packages/label` package.

## Usage

Run `npm run dev` to serve the API on port 3000.

`POST /todos` accepts `{"title":"Buy milk"}`. `GET /todos` lists todos,
`GET /todos/count` counts them, `POST /todos/:id/done` completes one, and
`DELETE /todos/:id` archives one. A new title is trimmed, spaced once and capitalised.

## Development

`npm test` runs node:test, including HTTP requests through Hono.
`tm check` reports the planted findings below. `fixes.toml` holds one repair per row.

| Decision row | Rule | Finding path |
| --- | --- | --- |
| Lock directory structure | `local/top-level-directories` | `scripts/seed.js` |
| Consistent file names | `local/use-case#count` | `src/inbound/use_cases/archive_todo` |
| Naming convention | `local/use-case-class-name` | `src/inbound/use_cases/list_todos/use_case.js` |
| Import direction | `local/no-framework-imports` | `src/inbound/use_cases/complete_todo/use_case.js` |
| Required tests | `local/tests-for-sources` | `src/domain/user/types.js` |
| Framework conventions | `local/async-handlers` | `src/routes/todos/routes.js` |
| Complexity limits | `local/too-many-parameters` | `src/domain/todo/titles.js` |
| Dependency graph | `local/use-case-import-cycles` | `src/inbound/use_cases/create_todo/use_case.js` |
| Banned patterns | `local/no-todo-comments` | `src/main.js` |
| Duplicate code | `local/duplicate-function-bodies` | `src/domain/todo/titles.js` |
| Idiomatic code | `local/no-loose-equality` | `src/outbound/todos/adapter.js` |
| JSON: dependencies pinned | `local/pinned-dependencies` | `package.json` |
| JSON: dependencies in order | `local/sorted-dependencies` | `package.json` |
| JSON: name and description | `local/package-identity` | `package.json` |
| Markdown: README sections | `local/readme-sections` | `README.md` |
