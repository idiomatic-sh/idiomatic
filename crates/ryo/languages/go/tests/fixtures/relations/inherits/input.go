package p

type Service struct {
	Base
	*Logger
	sync.Mutex
	Cache[string]
	name string
}

type Reader interface {
	io.Reader
	Close() error
}

type Number interface {
	~int | ~float64
	comparable
}

var anonymous struct {
	Embedded
}
