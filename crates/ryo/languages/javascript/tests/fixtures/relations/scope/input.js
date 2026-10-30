function outer(a) {
  for (let i = 0; i < a; i++) {
    try {
      run(i);
    } catch (error) {
      report(error);
    }
  }
  return () => {};
}
class Holder {
  static {
    init();
  }
}
for (const key in table) {}
for (const item of list) {}
