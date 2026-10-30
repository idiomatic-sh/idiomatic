package todos

import (
	"errors"
	"example.com/todo-api/src/domain/todo"
	"testing"
)

func TestStoreOwnsValues(t *testing.T) {
	store := New()
	first := store.Add("Milk")
	second := store.Add("Eggs")
	if first.ID == second.ID {
		t.Fatal("ids reused")
	}
	copy := store.All()
	copy[0].Title = "Changed"
	if store.All()[0].Title != "Milk" {
		t.Fatal("caller mutated store")
	}
	if _, err := store.Complete("missing"); !errors.Is(err, todo.ErrNotFound) {
		t.Fatal(err)
	}
	if !store.Archive(first.ID) || store.Archive(first.ID) {
		t.Fatal("archive result")
	}
	if len(store.All()) != 1 {
		t.Fatal(store.All())
	}
}
