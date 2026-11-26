"""Rows the two routes write differently, each judged once: a nonlocal name bound two functions up and a plain string inside an f-string interpolation."""
def method():
    func_name = None

    def decorator(func):
        def inner(*args, **kwargs):
            nonlocal func_name
            func_name = getattr(func, "__name__", None)
            return func(*args, **kwargs)

        return inner

    return decorator, func_name


def spec(reprs, a, b):
    return f"{'val': >{len(reprs[0])}}\n" + f"{a()}{'+'}{b}"
