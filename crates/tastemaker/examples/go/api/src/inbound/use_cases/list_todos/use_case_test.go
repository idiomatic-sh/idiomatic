package list_todos

import (
	"example.com/todo-api/src/outbound/todos"
	"testing"
)

func TestRun(t *testing.T) {
	store := todos.New()
	useCase := Lister{Store: store}
	added := store.Add("Milk")
	got := useCase.Run()
	if len(got) != 1 || got[0] != added {
		t.Fatal(got)
	}
}
