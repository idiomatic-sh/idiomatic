interface Props { label: string }
export const Widget = ({ label }: Props) => <section id="root">{label}<span {...{title: label}}>hello</span></section>;
export const Fragment = () => <><Widget label="hi" /></>;
function named() {}
class Box { open() {} }
const obj = { m() {}, n: function () {} };
