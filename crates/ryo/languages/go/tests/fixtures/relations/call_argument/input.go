package p

func run(settings *S, count int) {
	forward(settings, &count, make(), (*settings))
	obj.Go(count, 1)
	_ = []byte(count)
}
