def dataclass(cls):
    return cls


@dataclass
class Fake:
    value: int  # dead: value


print(Fake)
