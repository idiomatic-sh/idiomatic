class Outer:
    def method(self):
        def helper():
            return [x for x in range(3)]
        return helper

    class Inner:
        pass


squares = {k: k * k for k in range(5)}
evens = {n for n in squares if n % 2 == 0}
total = sum(v for v in squares.values())
pick = lambda seq: seq[0]
