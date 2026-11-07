package todos

import (
	"encoding/json"
	"errors"
	"example.com/todo-api/src/domain/todo"
	"example.com/todo-api/src/inbound/use_cases/archive_todo"
	"example.com/todo-api/src/inbound/use_cases/complete_todo"
	"example.com/todo-api/src/inbound/use_cases/create_todo"
	"example.com/todo-api/src/inbound/use_cases/list_todos"
	"io"
	"net/http"
	"strings"
)

func New(store todo.Store) http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /todos", func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, (list_todos.Lister{Store: store}).Run())
	})
	mux.HandleFunc("POST /todos", func(w http.ResponseWriter, r *http.Request) {
		var body struct {
			Title string `json:"title"`
		}
		decoder := json.NewDecoder(http.MaxBytesReader(w, r.Body, 1<<20))
		if decoder.Decode(&body) != nil || strings.TrimSpace(body.Title) == "" {
			http.Error(w, "a todo needs a title", http.StatusBadRequest)
			return
		}
		if decoder.Decode(new(any)) != io.EOF {
			http.Error(w, "one JSON object expected", http.StatusBadRequest)
			return
		}
		writeJSON(w, http.StatusCreated, (create_todo.CreateTodoUseCase{Store: store}).Run(body.Title))
	})
	mux.HandleFunc("POST /todos/{id}/done", func(w http.ResponseWriter, r *http.Request) {
		done, err := (complete_todo.CompleteTodoUseCase{Store: store}).Run(r.PathValue("id"))
		if errors.Is(err, todo.ErrNotFound) {
			http.Error(w, err.Error(), http.StatusNotFound)
			return
		}
		if err != nil {
			http.Error(w, "store failed", http.StatusInternalServerError)
			return
		}
		writeJSON(w, http.StatusOK, done)
	})
	mux.HandleFunc("DELETE /todos/{id}/archive", func(w http.ResponseWriter, r *http.Request) {
		if !(archive_todo.ArchiveTodoUseCase{Store: store}).Run(r.PathValue("id")) {
			http.Error(w, "no such todo", http.StatusNotFound)
			return
		}
		w.WriteHeader(http.StatusNoContent)
	})
	// planted: Framework conventions
	mux.HandleFunc("/todos/count", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet && r.Method != http.MethodHead {
			w.Header().Set("Allow", "GET, HEAD")
			w.WriteHeader(http.StatusMethodNotAllowed)
			return
		}
		writeJSON(w, http.StatusOK, map[string]int{"count": len(store.All())})
	})
	return mux
}

func writeJSON(w http.ResponseWriter, status int, value any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(value); err != nil {
		return
	}
}
