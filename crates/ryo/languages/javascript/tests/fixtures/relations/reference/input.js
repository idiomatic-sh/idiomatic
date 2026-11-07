import {read} from "fs";
const data = read(path.join(root, "x"));
console.log(data.length, data[key]);
function show(item) {
  return item.name;
}
