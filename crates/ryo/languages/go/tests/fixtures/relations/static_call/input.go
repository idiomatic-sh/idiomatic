package p

import "fmt"

func f(xs []func()) {
	fmt.Println("a")
	xs[0]()
	g()()
	_ = []byte("x")
	h[int](1)
	a.b.c.d()
	func() {}()
}
