// Every operator and keyword the language spells, in real source.
package spelled

func operators(a, b int, f float64, c complex128) (int, float64, complex128) {
	a += b
	a -= b
	a *= b
	a /= b
	a %= b
	a &= b
	a |= b
	a ^= b
	a <<= 1
	a >>= 1
	a &^= b
	a++
	a--
	x := a % b
	y := a << 1
	z := a >> 1
	w := a - b
	v := a / b
	u := a ^ b
	t := a &^ b
	ok := a == b || a <= b && a >= b
	_ = a < b
	_, _, _, _, _, _, _, _ = x, y, z, w, v, u, t, ok
	return a, f * 1.5, 2i + c
}

func flow(n int) int {
	for i := 0; i < n; i++ {
		if i == 1 {
			continue
		} else if i == 2 {
			goto done
		}
		switch i {
		case 3:
			fallthrough
		default:
			break
		}
		defer func() {}()
	}
done:
	return n
}
