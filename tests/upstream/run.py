"""Run vulture 2.16's own test suite against condor.

Usage (from the repository root, after `cargo build`):

    uv run --python 3.14 --with pytest python tests/upstream/run.py [pytest args]

The tests in `vulture-2.16/tests` are vulture's, unmodified, except for
`tests/__init__.py`, whose `call_vulture` helper runs the condor binary instead
of `python -m vulture` (see `PATCHED_INIT` below). They import a stand-in
`vulture` package (`shim/`) that keeps vulture's Python API and computes every
answer with condor through the `condor_testapi` bridge.

vulture is MIT-licensed, copyright Jendrik Seipp; see
`vulture-2.16/LICENSE.txt`.
"""

import os
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
UPSTREAM = HERE / "vulture-2.16"

PATCHED_INIT = '''\
import os
import pathlib
import subprocess
import sys

import pytest

from vulture import core

# condor: REPO is the vendored vulture 2.16 checkout.
REPO = pathlib.Path(os.environ["VULTURE_REPO"])
WHITELISTS = [
    str(path)
    for path in (REPO / "vulture" / "whitelists").glob("*.py")
    # Pint is incompatible with Python 3.13 (https://github.com/hgrecco/pint/issues/1969).
    if sys.version_info < (3, 13) or path.name != "pint_whitelist.py"
]


def call_vulture(args, **kwargs):
    # condor: run the condor binary instead of `python -m vulture`.
    return subprocess.call([os.environ["CONDOR_BIN"], *args], cwd=REPO, **kwargs)


def check(items_or_names, expected_names):
    """items_or_names must be a collection of Items or a set of strings."""
    try:
        assert sorted(item.name for item in items_or_names) == sorted(
            expected_names
        )
    except AttributeError:
        assert items_or_names == set(expected_names)


def check_unreachable(v, lineno, size, name):
    assert len(v.unreachable_code) == 1
    item = v.unreachable_code[0]
    assert item.first_lineno == lineno
    assert item.size == size
    assert item.name == name


def check_multiple_unreachable(v, checks):
    assert len(v.unreachable_code) == len(checks)
    for item, (lineno, size, name) in zip(v.unreachable_code, checks):
        assert item.first_lineno == lineno
        assert item.size == size
        assert item.name == name


@pytest.fixture
def v():
    return core.Vulture(verbose=True)
'''


def main():
    profile = os.environ.get("CONDOR_PROFILE", "debug")
    target = ROOT / "target" / profile
    work = ROOT / "target" / "upstream-tests"
    if work.exists():
        shutil.rmtree(work)
    shutil.copytree(UPSTREAM / "tests", work / "tests")
    (work / "tests" / "__init__.py").write_text(PATCHED_INIT)
    shutil.copy(HERE / "conftest_condor.py", work / "tests" / "conftest.py")
    env = {
        **os.environ,
        "PYTHONPATH": os.pathsep.join([str(HERE / "shim"), str(work)]),
        "CONDOR_TESTAPI": str(target / "condor_testapi"),
        "CONDOR_BIN": str(target / "condor"),
        "VULTURE_REPO": str(UPSTREAM),
        "PYTHONDONTWRITEBYTECODE": "1",
    }
    cmd = [sys.executable, "-m", "pytest", "-p", "no:cacheprovider", str(work / "tests"), *sys.argv[1:]]
    sys.exit(subprocess.call(cmd, env=env, cwd=work))


if __name__ == "__main__":
    main()
