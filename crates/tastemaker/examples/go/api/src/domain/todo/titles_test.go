package todo

import (
	"strings"
	"testing"
)

func TestNormaliseTitle(t *testing.T) {
	if got := NormaliseTitle("  Buy   milk "); got != "Buy milk" {
		t.Fatal(got)
	}
}
func TestDescribe(t *testing.T) {
	if got := Describe("Milk", true, "ann", "monday", "home"); !strings.HasPrefix(got, "Milk done ann monday") {
		t.Fatal(got)
	}
}
