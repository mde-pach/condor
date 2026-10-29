"""vulture.lines. These helpers measure CPython AST nodes, so they are
vulture's own code; the upstream size tests are redirected to condor by
`tests/upstream/conftest_condor.py`."""


def get_last_line_number(node):
    return node.end_lineno


def get_first_line_number(node):
    decorators = getattr(node, "decorator_list", [])
    if decorators:
        return decorators[0].lineno
    return node.lineno
