export function run(): void {}
export default class Shape {
  private width = 1;
  protected height = 2;
  public area(): number { return 0; }
  depth = 3;
}
export const a = 1, b = 2;
export interface Named { name: string }
export type Alias = string;
export enum Mode { On }
const hidden = 3;
export { hidden };
