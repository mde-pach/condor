import msgspec


class Event(msgspec.Struct):
    kind: str
    payload: dict

    def describe(self):  # dead: describe
        return self.kind


class Child(Event):
    extra: int


class Plain:
    plain_attr: str  # dead: plain_attr


print(Event, Child, Plain)
