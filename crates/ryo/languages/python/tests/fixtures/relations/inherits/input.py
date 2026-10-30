class Plain:
    pass


class Child(Base, mixins.Logging):
    pass


class Typed(Generic[T], Protocol):
    pass


class Meta(Base, metaclass=ABCMeta):
    pass


class Dynamic(bases[0], make_base(), *extra):
    pass
