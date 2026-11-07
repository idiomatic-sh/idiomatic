package p

type T struct{}

func f() {
	g := func() {
		h := func() {}
		_ = h
	}
	_ = g
}

func (t T) m() {}
