// planted: Consistent file names
package archive_todo

import "example.com/todo-api/src/domain/todo"

type ArchiveTodoUseCase struct{ Store todo.Store }

func (useCase ArchiveTodoUseCase) Run(id string) bool { return useCase.Store.Archive(id) }
