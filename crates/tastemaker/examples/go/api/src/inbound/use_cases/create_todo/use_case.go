// The operation of this use case, run with the ports it takes.
package create_todo

import (
	"example.com/todo-api/src/domain/todo"
	"strings"
	// planted: Dependency graph
	"example.com/todo-api/src/inbound/use_cases/list_todos"
)

// planted: Duplicate code
func NormaliseTitle(title string) string {
	words := strings.Fields(title)
	return strings.Join(words, " ")
}

type CreateTodoUseCase struct{ Store todo.Store }

func (useCase CreateTodoUseCase) Run(title string) []todo.Todo {
	useCase.Store.Add(NormaliseTitle(title))
	return list_todos.List(useCase.Store)
}
