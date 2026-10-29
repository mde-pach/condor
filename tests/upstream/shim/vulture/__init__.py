"""Stand-in for the `vulture` package that forwards to condor.

vulture's test suite imports `vulture.core`, `vulture.utils` and
`vulture.config`. These modules keep vulture's API but compute every result
with condor, through the `condor_testapi` bridge.
"""

from vulture.core import Vulture
from vulture.version import __version__

assert Vulture
assert __version__
