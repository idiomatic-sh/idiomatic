package todos

import (
	"example.com/todo-api/src/domain/todo"
	"github.com/google/uuid"
	"sync"
)

type MemoryStore struct {
	mu    sync.Mutex
	todos []todo.Todo
}

var _ todo.Store = (*MemoryStore)(nil)

func New() *MemoryStore { return &MemoryStore{todos: []todo.Todo{}} }

func (store *MemoryStore) All() []todo.Todo {
	store.mu.Lock()
	defer store.mu.Unlock()
	return append([]todo.Todo{}, store.todos...)
}

func (store *MemoryStore) Add(title string) todo.Todo {
	store.mu.Lock()
	defer store.mu.Unlock()
	added := todo.Todo{ID: uuid.NewString(), Title: title}
	store.todos = append(store.todos, added)
	return added
}

func (store *MemoryStore) Complete(id string) (todo.Todo, error) {
	store.mu.Lock()
	defer store.mu.Unlock()
	for index := range store.todos {
		if store.todos[index].ID == id {
			store.todos[index].Done = true
			return store.todos[index], nil
		}
	}
	return todo.Todo{}, todo.ErrNotFound
}

func (store *MemoryStore) Archive(id string) bool {
	store.mu.Lock()
	defer store.mu.Unlock()
	for index, found := range store.todos {
		if found.ID == id {
			store.todos = append(store.todos[:index], store.todos[index+1:]...)
			return true
		}
	}
	return false
}
