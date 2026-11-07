def full(a, b=1, /, c=2, *args, d, e=3, **kwargs):
    pass


def only_keyword(*, flag=False):
    pass


async def method(self, value: int):
    pass


square = lambda x, /, y=2, *, z: x * y * z
