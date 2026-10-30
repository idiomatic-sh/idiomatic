package archive_todo

import (
	"example.com/todo-api/src/outbound/todos"
	"testing"
)

func TestRun(t *testing.T) {
	store := todos.New()
	useCase := ArchiveTodoUseCase{Store: store}
	added := store.Add("Milk")
	if !useCase.Run(added.ID) || useCase.Run(added.ID) || len(store.All()) != 0 {
		t.Fatal("archive result")
	}
}
