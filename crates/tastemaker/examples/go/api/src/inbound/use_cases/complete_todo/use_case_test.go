package complete_todo

import (
	"example.com/todo-api/src/outbound/todos"
	"testing"
)

func TestRun(t *testing.T) {
	store := todos.New()
	useCase := CompleteTodoUseCase{Store: store}
	added := store.Add("Milk")
	got, err := useCase.Run(added.ID)
	if err != nil || !got.Done || got.ID != added.ID {
		t.Fatal(got, err)
	}
	if _, err := useCase.Run("missing"); err == nil {
		t.Fatal("missing todo completed")
	}
}
