"""Every operator and delimiter the language spells, in real source."""

def operators(a, b):
    c = a
    c -= b
    c &= b
    c ^= b
    c //= b
    c **= b
    c <<= b
    c >>= b
    c %= b
    c /= b
    c *= b
    c |= b
    c @= b
    d = a & b
    e = a ^ b
    f = a // b
    g = a << b
    h = a >> b
    i = -a
    j = ~a
    k = a @ b
    flags = (a < b, a <= b, a > b, a >= b, a != b)
    return c, d, e, f, g, h, i, j, k, flags


def delimiters(x):
    y = ...; z = x
    if (w := x) != y:
        pass
    return z, w


print(f"{1!r}")
