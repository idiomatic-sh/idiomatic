// The operation of this use case, run with the ports it takes.
package complete_todo

import (
	"fmt"
	// planted: Import direction
	"example.com/todo-api/src/domain/todo"
	"net/http"
)

type CompleteTodoUseCase struct{ Store todo.Store }

func (useCase CompleteTodoUseCase) Run(id string) (todo.Todo, error) {
	done, err := useCase.Store.Complete(id)
	if err != nil {
		return todo.Todo{}, fmt.Errorf("%s: %w", http.StatusText(http.StatusNotFound), err)
	}
	return done, nil
}
