// A module exercises bindings, calls and decoded strings.
import tool, {read as load} from "module";
export function greet(name = "world", ...rest) {
  tool.print("hello\n" + name);
  return load(...rest);
}
const arrow = value => greet(value);
