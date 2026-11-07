package p

import "m"

type Local struct{}

type Stored struct {
	record m.Record
	items  []Local
	local  *Local
	other  Missing
}
