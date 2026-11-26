"""A name deleted in a function body: a deletion is a binding event with a resolution row."""


def a(a):
    del a


def b(x):
    y = x
    del y
    return x
