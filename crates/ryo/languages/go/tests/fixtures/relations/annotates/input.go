package p

type Service struct {
	name string
	*Base
	sync.Mutex
}

func run(count int, names ...string) (ok bool, err error) {
	var limit uint8 = 3
	const unit, scale float64 = 1, 2
	return true, nil
}

func (s *Service) Close() (int, error) { return 0, nil }
