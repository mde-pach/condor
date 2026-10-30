import typing
from typing import NamedTuple, TypedDict


class Movie(TypedDict):
    title: str


class Pair(NamedTuple):
    left: int
    right: int

    def total(self):  # dead: total
        return 0


class Row(typing.NamedTuple):
    cell: int


class Plain:
    attr: int  # dead: attr


print(Movie, Pair, Row, Plain)
