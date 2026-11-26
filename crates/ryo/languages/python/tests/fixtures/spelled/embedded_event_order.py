"""The events of two embedded segments, one written while the table is written and one added afterwards: the old route numbers them in the order it embedded them, the walk in source order."""
from typing import Union

Alias = Union[type["Model"], int]


def use(model: type["Model"]):
    return model
