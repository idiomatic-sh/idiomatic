package todo

import "strings"

func NormaliseTitle(title string) string {
	words := strings.Fields(title)
	return strings.Join(words, " ")
}

// planted: Complexity limits
func Describe(title string, done bool, owner string, due string, tag string) string {
	state := "open"
	if done {
		state = "done"
	}
	return strings.Join([]string{title, state, owner, due, tag}, " ")
}
