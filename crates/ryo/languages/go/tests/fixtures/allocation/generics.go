package set

// Set is a set of comparable values.
type Set[E comparable] map[E]struct{}

// Add puts v in the set.
func (s Set[E]) Add(v E) { s[v] = struct{}{} }

// Has reports whether v is in the set.
func (s Set[E]) Has(v E) bool {
	_, ok := s[v]
	return ok
}

// Map applies f to each member of s.
func Map[E, F comparable](s Set[E], f func(E) F) Set[F] {
	out := make(Set[F], len(s))
	for v := range s {
		out.Add(f(v))
	}
	return out
}
