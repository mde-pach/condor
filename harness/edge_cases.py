"""Generate the edge-case corpus for the differential harness.

Each file exercises a rule that decides which dead code is reported or the
exit code (spec/compatibility.md).

Usage: python harness/edge_cases.py DEST

Writes hand-written files covering each rule of spec/compatibility.md, plus
seeded random modules that share a small pool of names (so that uses in one
file hide definitions in another). The output is deterministic.
"""

import os
import random
import sys
from pathlib import Path

CASES = {
    "syntax/match_stmt.py": """\
class Point:
    __match_args__ = ("x", "y")
    def __init__(self, x, y):
        self.x = x
        self.y = y

def where(p):
    match p:
        case Point(x=0, y=yy) if yy > 0:
            return "north"
        case Point(x=xx, y=0):
            return xx
        case {"k": v, **rest}:
            return rest
        case [first, *others]:
            return others
        case str() | bytes() as s:
            return s
        case _:
            unused_in_match = 1
            return None
""",
    "syntax/type_params.py": """\
from typing import Callable

def first[T](items: list[T]) -> T:
    return items[0]

def apply[*Ts, **P, R](f: Callable[P, R], *args: P.args, **kwargs: P.kwargs) -> R:
    return f(*args, **kwargs)

class Box[T = int, U: (str, bytes) = str]:
    def get(self) -> T: ...

type Pair[K] = tuple[K, K]
type Unused = dict[str, int]
""",
    "syntax/async_code.py": """\
import asyncio

async def fetch(session, url):
    async with session.get(url) as response:
        return await response.text()

async def crawl(urls):
    results = [await fetch(None, u) async for u in agen(urls)]
    async for item in agen(urls):
        print(item)
    return results

async def agen(xs):
    for x in xs:
        yield x
        await asyncio.sleep(0)
""",
    "syntax/fstrings.py": """\
width = 10
precision = 3
value = 3.14159
name = "x"
print(f"{value:{width}.{precision}}")
print(f"{name!r:>{width}} {value=} {f'{name}'}")
print(f"{'nested' + f'{width}'}")
print(f"{{literal}} {value:%}")
template = t"{name} {value:.2f}"
multi = f'''
{value
 + width}
'''
""",
    "syntax/misc_expr.py": """\
def f(a, /, b, *, c=1, **kw):
    if (n := len(kw)) > 1:
        return n
    lam = lambda x, y=2, *z, **w: x
    return lam

nums = [x for x in range(3) if x]
pairs = {k: v for k, v in zip("ab", nums)}
gen = (y for y in nums)
s = {*nums, *pairs}
d = {**pairs, "k": 1}
first, *rest = nums
del rest
obj = type("O", (), {})()
obj.attr = 1
del obj.attr
x = y = z = 0
print(x, f(1, b=2))
""",
    "decorators/decorators.py": """\
import functools

def deco(f):
    return f

class Widget:
    @property
    def size(self):
        return self._size

    @size.setter
    def size(self, value):
        self._size = value

    @staticmethod
    def make():
        return Widget()

    @classmethod
    def build(cls):
        return cls()

    @functools.lru_cache(maxsize=None)
    def cached(self):
        return 1

    @deco
    @functools.wraps(deco)
    def wrapped(self):
        pass

    @(lambda f: f)
    def weird(self):
        pass

    @registry[0]
    def indexed(self):
        pass

    @(
        deco
    )
    def multiline(self):
        pass

@deco
class Decorated:
    pass

registry = [deco]
""",
    "noqa/noqa_variants.py": """\
import os  # noqa
import sys  # noqa: F401
import json  # noqa: V104
import re  # NOQA:v104
import csv  #noqa
import ast  # noqa : V104
import glob  # noqa:V103 V104
import io  # noqa: E501,V104

def unused_one():  # noqa: V103
    pass

def unused_two():  # noqa: V101
    pass

class Unused:  # noqa
    pass

def f():
    local = 1  # noqa: F841
    other = 2  # noqa:V107
    return 3
    print("dead")  # noqa: V201

text = "# noqa"
""",
    "noqa/formfeed.py": "def a():\n    pass\n\x0c\ndef b():  # noqa\n    pass\n\x0c\x0c\ndef c():  # noqa\n    pass\nd = 1\x0b# noqa\n",
    "type_comments/valid.py": """\
from typing import Any, Dict, List, Tuple, Text

def plain(arg):
    # type: (Text) -> None
    pass

def inline(a,  # type: int
           b,  # type: Dict[str, int]
           ):
    # type: (...) -> List[int]
    return [a]

def header():  # type: () -> Tuple[int, int]
    return (1, 2)

values = {}  # type: Dict[str, Any]
for item in []:  # type: Any
    print(item)
with open("f") as fh:  # type: Any
    pass
print(values, plain, inline, header)
x = 1  # type: ignore
y = 2  # type: ignore[assignment]
""",
    "type_comments/ignored_variants.py": "f()  # type: ignore\ng()  #type:ignore[x]\nh()  # type: ignoreme\n",
    "usage/all_and_strings.py": """\
__all__ = ["exported", "also_exported"]
__all__ += ["not_counted"]
__all__: list = ["annotated"]

def exported(): pass
def also_exported(): pass
def not_counted(): pass
def annotated(): pass
def via_getattr(): pass
def via_hasattr(): pass
def via_getattr3(): pass
def too_many(): pass
def via_percent(): pass
def via_format(): pass
def via_format_attr(): pass

getattr(object, "via_getattr")
getattr(object, "via_getattr3", None)
getattr(object, "too_many", None, None)
hasattr(object, "via_hasattr")

def g():
    via_percent_local = 1
    via_format_local = 2
    obj = None
    print("%(via_percent_local)s" % locals())
    print("{via_format_local} {obj.via_format_attr[0]} {0} {}".format(**locals()))
    print("{broken".format(**locals()))
""",
    "usage/unicode_names.py": "\ufb01le = 1\nprint(file)\nnaïve = 2\ncafé = 3\nprint(cafe\u0301)\n",
    "usage/ignored_names.py": """\
_private = 1
__mangled = 2
__dunder__ = 3
object = 4
self = 5
_ = 6
cls = 7

class C:
    def __init__(self):
        pass
    def __repr__(self):
        return ""
    def method(cls):
        pass
    def __private_method(self):
        pass
""",
    "usage/imports.py": """\
from __future__ import annotations
import os.path
import xml.etree.ElementTree as ET
from collections import OrderedDict as OD, defaultdict
from . import sibling
from .. import parent
import sys, ast, string, logging, threading, unittest, enum, argparse
""",
    "pkg/__init__.py": "from .mod import thing\nfrom os import *\nimport json\n",
    "pkg/mod.py": "thing = 1\nother_thing = 2\n",
    "tests/test_things.py": """\
import pytest

def setup_module():
    pass

def test_one():
    assert True

def helper():
    pass

class TestGroup:
    def setup_method(self):
        pass
    def test_two(self):
        pass
    def helper(self):
        pass

class NotATestSuite:
    pass
""",
    "app/foo_test.py": "def test_x():\n    pass\nclass FooTest:\n    pass\n",
    "app/test_config.py": "class TestConfig:\n    pass\ndef test_y():\n    pass\n",
    "reach/flow.py": """\
def after_return():
    return 1
    print("dead")
    x = 2

def after_raise():
    raise ValueError()
    print("dead")

def loops(items):
    for item in items:
        if item:
            break
            print("dead")
        else:
            continue
            print("dead")
    while True:
        pass
    print("dead after infinite loop")

def loop_with_break():
    while 1:
        if check():
            break
    print("reachable")

def loop_else():
    while True:
        break
    else:
        print("unreachable else")

def never():
    while False:
        print("never")
    while 0:
        pass
    else:
        print("ok")

def conditions(x):
    if False:
        print("never")
    if 0:
        pass
    elif x:
        pass
    if True:
        print("always")
    if 1:
        pass
    else:
        print("never")
    if "":
        pass
    if []:
        pass
    if ():
        pass
    if {}:
        pass
    if set():
        pass
    if not False:
        pass
    if x and False:
        pass
    if x or True:
        pass
    if None:
        pass
    if -1:
        pass
    if 0.0:
        pass
    if 0j:
        pass
    if 1 - 1j:
        pass
    if b"":
        pass
    if ...:
        pass
    if __debug__:
        pass
    y = 1 if True else 2
    z = 1 if False else 2
    return y, z

def all_branches_return(x):
    if x:
        return 1
    elif x is None:
        return 2
    else:
        return 3
    print("dead")

def try_flow():
    try:
        return 1
    except ValueError:
        return 2
    else:
        print("unreachable else")
    print("dead")

def try_handler_falls():
    try:
        return 1
    except ValueError:
        pass
    print("reachable")

def try_star():
    try:
        raise ValueError()
    except* ValueError:
        return 1
    print("vulture does not analyse try*")

def with_flow(cm):
    with cm:
        return 1
        print("dead")

def match_flow(x):
    match x:
        case 1:
            return 1
            print("not analysed")

class K:
    return_in_class = 1

def check():
    return True
""",
    "reach/nested.py": """\
def outer():
    def inner():
        return 1
        dead = 2
    if True:
        return inner
    print("dead")

def chains(a):
    if a:
        raise ValueError()
    elif not a:
        return 1
    elif False:
        print("unsatisfiable elif")
    else:
        return 2
    print("dead")

async def async_flow(x):
    async for y in x:
        return y
        print("dead")
    async with x:
        raise ValueError()
        print("dead")
""",
    "encoding/latin1.py": None,  # written as bytes below
    "encoding/crlf.py": None,
    "encoding/cr_only.py": None,
    "encoding/bom.py": None,
    "encoding/invalid_utf8.py": None,
    "encoding/utf16.py": None,
    "encoding/null_byte.py": None,
    "encoding/bad_cookie.py": None,
    "errors/unclosed.py": "x = (1,\n",
    "errors/print_stmt.py": "print 'hello'\n",
    "hidden/.secret/mod.py": "def hidden_func():\n    pass\n",
    "dunder/module.py": "def __getattr__(name):\n    pass\n__version__ = '1'\n",
    "sizes/sizes.py": """\
@decorator
@other(
    1,
)
class Big:
    a = 1

    def method(self):
        return [
            1,
            2,
        ]
    # trailing comment

def small(): pass
""",
}

BYTES = {
    "encoding/latin1.py": b"# -*- coding: latin-1 -*-\nnom = '\xe9t\xe9'\ndef unused_latin(): pass\n",
    "encoding/crlf.py": b"def crlf_unused():\r\n    return 1\r\n    print('dead')\r\nx = 1  # noqa\r\n",
    "encoding/cr_only.py": b"def cr_unused():\r    pass\r",
    "encoding/bom.py": b"\xef\xbb\xbfdef bom_unused():\n    pass\n",
    "encoding/invalid_utf8.py": b"x = '\xff\xfe'\n",
    "encoding/utf16.py": "x = 1\n".encode("utf-16"),
    "encoding/null_byte.py": b"x = 1\ny = '\x00'\n",
    "encoding/bad_cookie.py": b"# coding: no-such-codec\nx = 1\n",
}


# ----- random modules ---------------------------------------------------------

NAMES = [f"name{i}" for i in range(40)] + ["helper", "process", "value", "Thing", "_private", "data"]


def random_module(rng, index):
    lines = []
    indent = ""

    def emit(s):
        lines.append(indent + s)

    for _ in range(rng.randint(5, 25)):
        kind = rng.random()
        n1, n2 = rng.choice(NAMES), rng.choice(NAMES)
        if kind < 0.15:
            emit(f"{n1} = {n2}")
        elif kind < 0.3:
            deco = rng.choice(["", "@property\n", "@staticmethod\n", "@app.route('/x')\n", "@pytest.fixture\n"])
            params = ", ".join(rng.sample(NAMES, rng.randint(0, 3)))
            body = rng.choice(
                [
                    f"return {n2}",
                    f"return {n2}\n    {n1}()",
                    f"if False:\n        {n2}()\n    return {n1}",
                    f"while True:\n        {n2}()",
                    f"{n2}.{n1} = 1",
                    f"print(getattr({n2}, '{rng.choice(NAMES)}'))",
                ]
            )
            for line in (deco + f"def {n1}({params}):\n    {body}").splitlines():
                emit(line)
        elif kind < 0.42:
            emit(f"class {n1.capitalize()}:")
            emit(f"    def {n2}(self, {rng.choice(NAMES)}):")
            emit(f"        self.{rng.choice(NAMES)} = {rng.choice(NAMES)}")
            emit("    @property")
            emit(f"    def {rng.choice(NAMES)}(self):")
            emit(f"        return self.{rng.choice(NAMES)}")
        elif kind < 0.5:
            emit(f"from mod{rng.randint(0, 9)} import {n1} as {n2}")
        elif kind < 0.58:
            emit(f"import {rng.choice(['os', 'sys', 'ast', 'json', 'string', 'collections'])}")
        elif kind < 0.66:
            emit(f"print({n1}.{n2}, {rng.choice(NAMES)}({n1}))")
        elif kind < 0.72:
            emit(f"__all__ = [{', '.join(repr(n) for n in rng.sample(NAMES, 2))}]")
        elif kind < 0.78:
            emit(f"{n1} = 1  # noqa{rng.choice(['', ': V107', ': V103', ':F841'])}")
        elif kind < 0.84:
            emit(f"try:\n    {n1}()\nexcept Exception:\n    {n2} = 1\nelse:\n    pass")
        elif kind < 0.9:
            emit(f"{n1} = {rng.choice(['1 if True else 2', '0 if 0 else 1', 'lambda q: q'])}")
        else:
            emit(f"for {n1} in {n2}:\n    if {n1}:\n        break\n        {n2}()")
    return "\n".join(lines) + "\n"


def main():
    dest = Path(sys.argv[1])
    dest.mkdir(parents=True, exist_ok=True)
    for name, text in CASES.items():
        path = dest / name
        path.parent.mkdir(parents=True, exist_ok=True)
        if text is None:
            path.write_bytes(BYTES[name])
        else:
            path.write_text(text, encoding="utf-8")
    rng = random.Random(20261029)
    for i in range(300):
        path = dest / "random" / f"pkg{i % 10}" / f"mod{i}.py"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(random_module(rng, i))
    # A symlinked directory (not followed) and a symlinked file (included).
    link_dir = dest / "links" / "dir_link"
    link_dir.parent.mkdir(parents=True, exist_ok=True)
    os.symlink(dest / "syntax", link_dir)
    os.symlink(dest / "pkg" / "mod.py", dest / "links" / "file_link.py")
    (dest / "pyproject.toml").write_text('[tool.vulture]\nignore_names = ["name3*"]\n')


if __name__ == "__main__":
    main()
