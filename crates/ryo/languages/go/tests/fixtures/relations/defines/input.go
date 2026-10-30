package p

import (
	"fmt"
	str "strings"
	. "math"
	_ "embed"
	"gopkg.in/yaml.v3"
)

const limit, _ = 1, 2

var count, total int

type Point struct {
	X, Y  int
	Label string
	*Embedded
	fmt.Stringer
}

type Shape interface {
	Area() float64
	fmt.Stringer
}

type Alias = Point

type Pair[K comparable, V any] struct{}

type ID int

func run(a int, _ string, rest ...int) (n int, err error) {
	x := 1
	for i, v := range rest {
		_, _ = i, v
	}
	switch t := any(a).(type) {
	case int:
		_ = t
	}
	f := func(inner int) {}
Outer:
	for {
		break Outer
	}
	return
}

func (p *Point) Move(dx int) {}

func Map[T, U any](xs []T) []U { return nil }
