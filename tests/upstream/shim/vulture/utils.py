"""vulture.utils, answered by condor."""

import ast
import os
import pathlib
from enum import IntEnum

from vulture import _bridge


class VultureInputException(Exception):
    pass


class ExitCode(IntEnum):
    NoDeadCode = 0
    InvalidInput = 1
    InvalidCmdlineArguments = 2
    DeadCode = 3


def _condition(node):
    return _bridge.call("condition", expr=ast.unparse(node))


def condition_is_always_false(condition):
    return _condition(condition)["always_false"]


def condition_is_always_true(condition):
    return _condition(condition)["always_true"]


def format_path(path):
    result = _bridge.call("format_path", path=str(path), cwd=os.getcwd())
    return pathlib.Path(result["path"])


def get_decorator_name(decorator):
    return _bridge.call("decorator_name", expr=ast.unparse(decorator))["name"]
