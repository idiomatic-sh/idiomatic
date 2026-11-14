function f(x) {
  while (true) {}
  if (x) { a(); } else { b(); }
  c();
}
function h() { return; foo(); }
function t() { try { a(); } catch { b(); } finally { c(); } }
function l() { outer: for (;;) { break outer; } }
function e(y) { y(); }
