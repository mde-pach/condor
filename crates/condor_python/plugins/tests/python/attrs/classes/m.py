import attr
import attrs


@attr.s
class Old:
    x = attr.ib()
    y = attr.ib(default=0)
    CONSTANT = 1  # dead: CONSTANT

    def helper(self):  # dead: helper
        return 1


@attrs.define
class New:
    a: int
    b: str = ""


@attrs.frozen
class Frozen:
    value: int


class Plain:
    x = 1  # dead: x


print(Old, New, Frozen, Plain)
