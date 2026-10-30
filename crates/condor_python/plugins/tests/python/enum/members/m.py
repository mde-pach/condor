import enum
from enum import Enum, IntEnum


class Color(Enum):
    RED = 1
    GREEN = 2

    def describe(self):  # dead: describe
        return self.name


class Level(enum.IntFlag):
    LOW = 1


class Prio(IntEnum):
    HIGH = 1


class Base(Enum):
    pass


class Derived(Base):
    MEMBER = 1


class NotEnum:
    CONSTANT = 1  # dead: CONSTANT


print(Color, Level, Prio, Derived, NotEnum)
