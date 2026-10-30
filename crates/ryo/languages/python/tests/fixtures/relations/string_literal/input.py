"""The module's docstring."""

__all__ = ["export", 'other']
plain = "line\n" 'joined'
raw = r"\d+"
static = f"no {{braces}} here"
nested = f"outer {f'inner {value}'} done"
template = t"hello {name}"
data = b"bytes are no strings"
multi = """
many lines
"""
plugin = importlib.import_module("plugins.extra")
loaded = __import__("os")
