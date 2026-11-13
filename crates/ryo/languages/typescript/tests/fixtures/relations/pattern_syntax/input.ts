const a = /^a|b$/u;
const b = /(?=x)(?<n>c)+\k<n>\1(?:d)./;
const c = /[^\d\p{L}a-z]\b/u;
const d = /[\q{ab}]/v;
const e = new RegExp("\u{1F44D}", "u");
