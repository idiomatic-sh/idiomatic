import tool, {read as load} from "module";
import * as path from "path";
const [first, {second}] = load();
let later;
var old = 1;
function run(value) {
  return value;
}
class Store {
  items = [];
  #secret = 1;
  add(item) {}
  [computed]() {}
}
outer: for (;;) {
  break outer;
}
