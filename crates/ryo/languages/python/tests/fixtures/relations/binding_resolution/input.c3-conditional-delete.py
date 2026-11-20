def f():
    global x
    x = 1
    if c:
        del x
    return x
