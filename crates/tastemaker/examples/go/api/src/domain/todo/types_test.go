package todo

import (
	"encoding/json"
	"testing"
)

func TestTodoJSON(t *testing.T) {
	bytes, err := json.Marshal(Todo{ID: "one", Title: "Milk"})
	if err != nil {
		t.Fatal(err)
	}
	if string(bytes) != `{"id":"one","title":"Milk","done":false}` {
		t.Fatal(string(bytes))
	}
}
