module example.com/todo-api

go 1.22

require (
	github.com/google/uuid v1.6.0
	example.com/todo-label v0.0.0
)

replace example.com/todo-label => ./packages/label
