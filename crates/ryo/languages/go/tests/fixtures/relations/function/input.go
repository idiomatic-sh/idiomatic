package p

type T struct{}

func f() {}

func (t *T) set(a int) { _ = a }

func g[E any](x E) E { return x }

func external(a int) int

var literal = func() {}
