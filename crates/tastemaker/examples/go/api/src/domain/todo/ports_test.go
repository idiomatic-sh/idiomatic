package todo_test

import (
	"example.com/todo-api/src/domain/todo"
	"example.com/todo-api/src/outbound/todos"
	"testing"
)

func TestStorePort(t *testing.T) {
	var store todo.Store = todos.New()
	added := store.Add("Milk")
	done, err := store.Complete(added.ID)
	if err != nil || !done.Done {
		t.Fatal(done, err)
	}
	if !store.Archive(added.ID) || len(store.All()) != 0 {
		t.Fatal(store.All())
	}
}
