// The operation of this use case, run with the ports it takes.
package list_todos

import "example.com/todo-api/src/domain/todo"

// planted: Naming convention
type Lister struct{ Store todo.Store }

func (useCase Lister) Run() []todo.Todo { return List(useCase.Store) }

func List(store todo.Store) []todo.Todo { return store.All() }
