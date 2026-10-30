// planted: Consistent file names
export class ArchiveTodoUseCase {
  constructor(store) { this.store = store; }
  async run(id) { return this.store.archive(id); }
}
