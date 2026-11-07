import os
import os.path
import xml.etree.ElementTree as ET
from collections import OrderedDict, defaultdict as dd
from . import sibling
from .pkg import thing as other
from ..parent.mod import name
from m import *

try:
    import ujson as json
except ImportError:
    import json

if TYPE_CHECKING:
    from typing import Protocol


def lazy():
    import importlib
    return importlib
