"""Added to vulture's test package as `conftest.py`.

`test_size.py` measures nodes of CPython's `ast` with vulture's `lines`
helpers, which says nothing about condor. Its `check_size` helper is replaced
so that every example is scanned by condor and the size condor reports for
class `Foo` is checked instead. The examples and expected sizes are vulture's.
"""

import pytest

from vulture import core


def condor_check_size(example, size):
    v = core.Vulture(verbose=False)
    v.scan(example)
    foo = [item for item in v.defined_classes if item.name == "Foo"]
    assert foo, 'Failed to find class "Foo" in code'
    assert foo[0].size == size


@pytest.fixture(autouse=True)
def _condor_sizes(request, monkeypatch):
    if request.module.__name__.endswith("test_size"):
        monkeypatch.setattr(request.module, "check_size", condor_check_size)


# Upstream tests condor deliberately does not pass, with the reason. Each one
# is explained in spec/divergences.md or spec/improvements.md.
EXPECTED_FAILURES = {
    "test_invalid_type_comment": (
        "D3: condor does not validate type comments against CPython's grammar; "
        "a malformed one does not make the file invalid input"
    ),
    "test_match_class_simple": "I2: dataclass fields are treated as used",
    "test_match_class_embedded": "I2: dataclass fields are treated as used",
    "test_match_enum": "I2: enum members are treated as used",
}


def pytest_collection_modifyitems(items):
    for item in items:
        reason = EXPECTED_FAILURES.get(item.name)
        if reason:
            item.add_marker(pytest.mark.xfail(reason=reason, strict=True))
