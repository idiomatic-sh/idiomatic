package p

const name = "ryo"

var title, kind = "a", other()

func run() {
	greeting := "hi"
	greeting = "bye"
	count := 1
	count += 2
	left, right := "x", 3
	_, tail := 4, `raw`
	var (
		raw = `r`
	)
	f := func() { inner := "in" }
}
