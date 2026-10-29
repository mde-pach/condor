"""vulture.config, answered by condor."""

import os
import sys

from vulture import _bridge

DEFAULTS = {
    "config": "pyproject.toml",
    "min_confidence": 0,
    "paths": [],
    "exclude": [],
    "ignore_decorators": [],
    "ignore_names": [],
    "make_whitelist": False,
    "sort_by_size": False,
    "verbose": False,
}


class InputError(Exception):
    def __init__(self, message):
        self.message = message


def _exit(result):
    sys.stdout.write(result["exit"]["stdout"])
    sys.stderr.write(result["exit"]["stderr"])
    raise SystemExit(result["exit"]["code"])


def _check_input_config(data):
    result = _bridge.call("check_input_config", data=data)
    if "input_error" in result:
        raise InputError(result["input_error"])


def _parse_toml(infile):
    result = _bridge.call("parse_toml", content=infile.read().decode("utf-8"))
    if "input_error" in result:
        raise InputError(result["input_error"])
    if "toml_error" in result:
        raise ValueError(result["toml_error"])
    return result["ok"]


def _parse_args(args=None):
    argv = sys.argv[1:] if args is None else list(args)
    result = _bridge.call("parse_args", argv=argv)
    if "exit" in result:
        _exit(result)
    return result["ok"]


def make_config(argv=None, tomlfile=None):
    argv = sys.argv[1:] if argv is None else list(argv)
    toml = tomlfile.read().decode("utf-8") if tomlfile else None
    result = _bridge.call("make_config", argv=argv, toml=toml, cwd=os.getcwd())
    if "exit" in result:
        _exit(result)
    sys.stdout.write(result.get("stdout", ""))
    if "input_error" in result:
        raise InputError(result["input_error"])
    return result["ok"]
