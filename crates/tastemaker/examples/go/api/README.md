# Todo API — Go

A Go 1.22+ API using net/http and method-aware ServeMux patterns.
Routes translate HTTP, use cases take the todo.Store port, and the memory adapter
owns its data behind a mutex. UUIDs come from the registry dependency; the display
label comes from the local module in packages/label.

## Install

Install Go 1.22 or newer. `go test ./...` fetches the module dependencies.

## Usage

Run `go run ./src` to listen on port 3000.

`POST /todos` accepts `{"title":"Buy milk"}` and returns the current list.
`GET /todos` lists todos, `GET /todos/count` counts them,
`POST /todos/{id}/done` completes one, and `DELETE /todos/{id}/archive` archives one.
Titles are trimmed and whitespace is collapsed. Each handler instance owns its store.

## Development

`go vet ./...` and `go test ./...` check the sample; the route tests send HTTP
requests through httptest. `tm check` reports the findings below.
`fixes.toml` gives one independent repair per row.

Go already rejects import cycles, so the dependency-graph row checks independence
between use cases. Go requirements are minimum versions, not exact pins. The shared
pinning rule therefore does not apply here. The shared sorting rule does: the
`require` block in `go.mod` is planted out of order.

| Decision row | Rule | Finding path |
| --- | --- | --- |
| Lock directory structure | `local/top-level-directories` | `scripts/seed.go` |
| Consistent file names | `local/use-case#count` | `src/inbound/use_cases/archive_todo` |
| Naming convention | `local/use-case-type-name` | `src/inbound/use_cases/list_todos/use_case.go` |
| Import direction | `local/no-framework-imports` | `src/inbound/use_cases/complete_todo/use_case.go` |
| Required tests | `local/tests-for-sources` | `src/domain/user/types.go` |
| Framework conventions | `local/method-routing-patterns` | `src/routes/todos/routes.go` |
| Complexity limits | `local/too-many-parameters` | `src/domain/todo/titles.go` |
| Dependency graph | `local/independent-use-cases` | `src/inbound/use_cases/create_todo/use_case.go` |
| Banned patterns | `local/no-todo-comments` | `src/main.go` |
| Duplicate code | `local/duplicate-function-bodies` | `src/domain/todo/titles.go` |
| Idiomatic code | `local/no-dot-imports` | `src/main.go` |
| Markdown: README sections | `local/readme-sections` | `README.md` |
| go.mod: dependencies in order | `local/sorted-dependencies` | `go.mod` |
