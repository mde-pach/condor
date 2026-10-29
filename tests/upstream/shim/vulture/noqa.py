"""vulture.noqa, answered by condor."""

from vulture import _bridge

NOQA_CODE_MAP = _bridge.call("constants")["noqa_code_map"]


class _Match:
    def __init__(self, result):
        self._result = result

    def groupdict(self):
        return {"codes": self._result["codes"]}


class _Regexp:
    def search(self, line):
        result = _bridge.call("noqa_search", line=line)
        return _Match(result) if result["match"] else None


NOQA_REGEXP = _Regexp()


def _parse_error_codes(match):
    return match._result["parsed"]
