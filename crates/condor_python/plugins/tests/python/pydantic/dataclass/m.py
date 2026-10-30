from pydantic.dataclasses import dataclass


@dataclass
class Item:
    price: int
    CONSTANT = 1  # dead: CONSTANT


print(Item)
