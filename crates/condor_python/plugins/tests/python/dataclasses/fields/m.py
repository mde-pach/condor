import dataclasses
from dataclasses import dataclass, field
from typing import ClassVar


@dataclass
class User:
    name: str
    tags: list = field(default_factory=list)
    LIMIT = 10  # dead: LIMIT
    registry: ClassVar[dict] = {}  # dead: registry

    def helper(self):  # dead: helper
        return 1


@dataclasses.dataclass(frozen=True)
class Point:
    x: int
    y: int


class Admin(User):
    level: int


class Plain:
    field_like: int  # dead: field_like


print(User, Point, Admin, Plain)
