"""vulture.core, answered by condor.

A `Vulture` object records its `scan` and `scavenge` calls. Each call replays
all of them through the condor bridge, prints the output of the new call, and
keeps condor's view of the defined, used and unused code.
"""

import os
import sys
from pathlib import Path

from vulture import _bridge
from vulture.utils import ExitCode

DEFAULT_CONFIDENCE = 60

ERROR_CODES = _bridge.call("constants")["error_codes"]


class Item:
    def __init__(self, data):
        self.name = data["name"]
        self.typ = data["typ"]
        self.filename = Path(data["filename"])
        self.first_lineno = data["first_lineno"]
        self.last_lineno = data["last_lineno"]
        self.message = data["message"]
        self.confidence = data["confidence"]
        self._report = data["report"]
        self._whitelist = data["whitelist"]

    @property
    def size(self):
        assert self.last_lineno >= self.first_lineno
        return self.last_lineno - self.first_lineno + 1

    def get_report(self, add_size=False):
        return self._report

    def get_whitelist_string(self):
        return self._whitelist

    def _tuple(self):
        return self.filename, self.first_lineno, self.name

    def __repr__(self):
        return repr(self.name)

    def __eq__(self, other):
        return self._tuple() == other._tuple()

    def __hash__(self):
        return hash(self._tuple())


class Vulture:
    def __init__(self, verbose=False, ignore_names=None, ignore_decorators=None):
        self.verbose = verbose
        self.ignore_names = ignore_names or []
        self.ignore_decorators = ignore_decorators or []
        self.exit_code = ExitCode.NoDeadCode
        self._actions = []
        self._printed = 0
        self._state = None

    def _session(self, query=None):
        result = _bridge.call(
            "session",
            verbose=self.verbose,
            ignore_names=list(self.ignore_names),
            ignore_decorators=list(self.ignore_decorators),
            cwd=os.getcwd(),
            actions=self._actions,
            query=query or {},
        )
        for out in result["outputs"][self._printed :]:
            sys.stdout.write(out["stdout"])
            sys.stderr.write(out["stderr"])
        self._printed = len(result["outputs"])
        if result["exit_code"] == ExitCode.InvalidInput:
            self.exit_code = ExitCode.InvalidInput
        self._state = result
        if result["exit"]:
            raise SystemExit(result["exit"].get("message", result["exit"]["code"]))
        return result

    def scan(self, code, filename=""):
        self._actions.append({"scan": {"code": code, "filename": str(filename)}})
        self._session()

    def scavenge(self, paths, exclude=None):
        self._actions.append(
            {"scavenge": {"paths": [str(p) for p in paths], "exclude": list(exclude or [])}}
        )
        self._session()

    def get_unused_code(self, min_confidence=0, sort_by_size=False):
        if not 0 <= min_confidence <= 100:
            raise ValueError("min_confidence must be between 0 and 100.")
        query = {"unused_code": {"min_confidence": min_confidence, "sort_by_size": sort_by_size}}
        return [Item(i) for i in self._session(query)["query"]["items"]]

    def report(self, min_confidence=0, sort_by_size=False, make_whitelist=False):
        query = {
            "report": {
                "min_confidence": min_confidence,
                "sort_by_size": sort_by_size,
                "make_whitelist": make_whitelist,
            }
        }
        result = self._session(query)["query"]
        sys.stdout.write(result["output"]["stdout"])
        if result["code"] == ExitCode.DeadCode:
            self.exit_code = ExitCode.DeadCode
        return self.exit_code

    def _log(self, *args, file=None, force=False):
        if self.verbose or force:
            file = file or sys.stdout
            try:
                print(*args, file=file)
            except UnicodeEncodeError:
                x = " ".join(map(str, args))
                print(x.encode(), file=file)

    def _items(self, group, typ):
        if self._state is None:
            self._session()
        return [Item(i) for i in self._state[group][typ]]

    @property
    def used_names(self):
        if self._state is None:
            self._session()
        return set(self._state["used_names"])

    defined_attrs = property(lambda self: self._items("defined", "attribute"))
    defined_classes = property(lambda self: self._items("defined", "class"))
    defined_funcs = property(lambda self: self._items("defined", "function"))
    defined_imports = property(lambda self: self._items("defined", "import"))
    defined_methods = property(lambda self: self._items("defined", "method"))
    defined_props = property(lambda self: self._items("defined", "property"))
    defined_vars = property(lambda self: self._items("defined", "variable"))
    unreachable_code = property(lambda self: self._items("defined", "unreachable_code"))
    unused_attrs = property(lambda self: self._items("unused", "attribute"))
    unused_classes = property(lambda self: self._items("unused", "class"))
    unused_funcs = property(lambda self: self._items("unused", "function"))
    unused_imports = property(lambda self: self._items("unused", "import"))
    unused_methods = property(lambda self: self._items("unused", "method"))
    unused_props = property(lambda self: self._items("unused", "property"))
    unused_vars = property(lambda self: self._items("unused", "variable"))
