package p

type T struct{}

func (t *T) set(a, b int, rest ...string) (n int, err error) { return }

func (T) anon(int, string) error { return nil }

func f[E any](x E) (E, bool) {
	g := func(p int) {}
	_ = g
	return x, true
}
