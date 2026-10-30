package todo

type Store interface {
	All() []Todo
	Add(title string) Todo
	Complete(id string) (Todo, error)
	Archive(id string) bool
}
