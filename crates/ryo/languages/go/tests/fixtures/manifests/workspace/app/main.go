package main

import (
	"fmt"

	"example.com/app/internal/greet"
	"example.com/core"
)

func main() { fmt.Println(greet.Hello(), core.Name) }
