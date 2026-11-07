package main

import (
	// planted: Idiomatic code
	. "fmt"
	"log"
	"net/http"

	"example.com/todo-api/src/outbound/todos"
	routes "example.com/todo-api/src/routes/todos"
	"example.com/todo-label"
)

// planted: Banned patterns
// TODO: make the listen address configurable
const address = ":3000"

func banner() string { return Sprintf("%s on %s", label.Name, address) }

func main() {
	log.Print(banner())
	log.Fatal(http.ListenAndServe(address, routes.New(todos.New())))
}
