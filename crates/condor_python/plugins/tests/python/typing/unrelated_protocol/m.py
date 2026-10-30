class Protocol:
    pass


class NotAProtocol(Protocol):
    def method(self, unused_param):  # dead: unused_param
        return 1


print(NotAProtocol().method)
