package create_todo

import (
	"example.com/todo-api/src/outbound/todos"
	"testing"
)

func TestRun(t *testing.T) {
	store := todos.New()
	useCase := CreateTodoUseCase{Store: store}
	got := useCase.Run(" Buy   milk ")
	if len(got) != 1 || got[0].Title != "Buy milk" {
		t.Fatal(got)
	}
}
