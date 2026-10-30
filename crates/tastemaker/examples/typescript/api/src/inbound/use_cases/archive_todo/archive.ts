// planted: Consistent file names (no use_case.ts here)
export class ArchiveTodoUseCase {
  async run(id: number): Promise<number> {
    return id;
  }
}
