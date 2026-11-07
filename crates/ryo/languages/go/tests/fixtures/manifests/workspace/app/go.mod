module example.com/app

go 1.24

require (
	example.com/core v1.2.0
	example.com/util v0.1.0 // indirect
	github.com/other/lib v1.0.0
)

replace github.com/other/lib v1.0.0 => github.com/fork/lib v1.0.1
