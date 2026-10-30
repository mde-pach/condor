from typing import Protocol, overload, override


class Reader(Protocol):
    def read(self, size): ...

    def close(self, force):
        return None


class FileReader(Reader):
    def read(self, size):  # dead: size
        return 0

    def seek(self, offset):  # dead: offset
        return 0


class Base:
    def run(self, job):  # dead: job
        return 1


class Child(Base):
    @override
    def run(self, job):
        return 2


@overload
def get(key: int) -> int: ...
@overload
def get(key: str) -> str: ...
def get(key, default=None):  # dead: default
    return key


def concrete(arg):  # dead: concrete, arg
    pass


print(Reader.read, Reader.close, FileReader().read, FileReader().seek, Child().run, get)
