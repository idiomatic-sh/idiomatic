module example.com/m

go 1.22

require example.com/one v1.0.0

require (
	example.com/b v1.2.0
	example.com/a v0.3.1 // indirect
)

exclude example.com/c v0.0.1
