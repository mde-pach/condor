import attr


def first():
    @attr.s
    class C:
        x = attr.ib()

    class D(C):
        y = attr.ib()

    return D


def second():
    class C:
        plain = 1  # dead: plain

    class D(C):
        other = 2  # dead: other

        def method(self, unused):  # dead: method, unused
            return 1

    return D


print(first, second)
