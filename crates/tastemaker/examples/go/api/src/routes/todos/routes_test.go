package todos

import (
	"encoding/json"
	"example.com/todo-api/src/domain/todo"
	store "example.com/todo-api/src/outbound/todos"
	"net/http"
	"net/http/httptest"
	"reflect"
	"strings"
	"testing"
)

func request(handler http.Handler, method, path, body string) *httptest.ResponseRecorder {
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, httptest.NewRequest(method, path, strings.NewReader(body)))
	return recorder
}

func TestTodoLifecycleThroughHTTP(t *testing.T) {
	handler := New(store.New())
	created := request(handler, "POST", "/todos", `{"title":"  Buy   milk "}`)
	if created.Code != 201 {
		t.Fatalf("create: %d %s", created.Code, created.Body)
	}
	if created.Header().Get("Content-Type") != "application/json" {
		t.Fatal(created.Header())
	}
	var added []todo.Todo
	if err := json.Unmarshal(created.Body.Bytes(), &added); err != nil {
		t.Fatal(err)
	}
	if len(added) != 1 || added[0].ID == "" || added[0].Title != "Buy milk" || added[0].Done {
		t.Fatal(added)
	}
	listed := request(handler, "GET", "/todos", "")
	var todos []todo.Todo
	if err := json.Unmarshal(listed.Body.Bytes(), &todos); err != nil {
		t.Fatal(err)
	}
	if listed.Code != 200 || !reflect.DeepEqual(todos, added) {
		t.Fatal(listed, todos)
	}
	counted := request(handler, "GET", "/todos/count", "")
	if counted.Code != 200 || strings.TrimSpace(counted.Body.String()) != `{"count":1}` {
		t.Fatal(counted)
	}
	done := request(handler, "POST", "/todos/"+added[0].ID+"/done", "")
	var completed todo.Todo
	if err := json.Unmarshal(done.Body.Bytes(), &completed); err != nil {
		t.Fatal(err)
	}
	if done.Code != 200 || !completed.Done || completed.ID != added[0].ID {
		t.Fatal(done, completed)
	}
	if request(handler, "POST", "/todos/missing/done", "").Code != 404 {
		t.Fatal("missing todo completed")
	}
	if request(handler, "DELETE", "/todos/"+added[0].ID+"/archive", "").Code != 204 {
		t.Fatal("archive failed")
	}
	if request(handler, "DELETE", "/todos/"+added[0].ID+"/archive", "").Code != 404 {
		t.Fatal("archived twice")
	}
	if strings.TrimSpace(request(handler, "GET", "/todos", "").Body.String()) != "[]" {
		t.Fatal("archive retained todo")
	}
	if strings.TrimSpace(request(New(store.New()), "GET", "/todos", "").Body.String()) != "[]" {
		t.Fatal("state leaked")
	}
}

func TestBadRequestsDoNotMutateStore(t *testing.T) {
	handler := New(store.New())
	for _, body := range []string{"{", "null", `{"title":1}`, `{"title":" "}`, `{"title":"Milk"}{}`} {
		if got := request(handler, "POST", "/todos", body); got.Code != 400 {
			t.Fatalf("%q: %d", body, got.Code)
		}
	}
	if got := request(handler, "GET", "/todos", "").Body.String(); strings.TrimSpace(got) != "[]" {
		t.Fatal(got)
	}
	for _, path := range []string{"/todos", "/todos/count", "/todos/id/done"} {
		if got := request(handler, "PUT", path, ""); got.Code != 405 {
			t.Fatalf("%s: %d", path, got.Code)
		}
	}
	if request(handler, "GET", "/unknown", "").Code != 404 {
		t.Fatal("unknown route matched")
	}
}
