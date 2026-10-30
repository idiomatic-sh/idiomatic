package p

import "fmt"

var plain = "plain"
var escaped = "tab\there é \x41 \101"
var raw = `raw \n text`
var bytes = "\xff"
var ch = 'c'

type T struct {
	Name string `json:"name"`
}

var _ = fmt.Sprint
