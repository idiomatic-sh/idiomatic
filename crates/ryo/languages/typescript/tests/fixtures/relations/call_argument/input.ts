function run(settings: S, count: number) {
  forward<S>(settings, ...count, make(), settings.x);
  obj.go(count, 1);
  new Thing(settings);
}
