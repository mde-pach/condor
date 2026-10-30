import unittest


class Base(unittest.TestCase):
    def check(self, value):
        return value


class StorageTests(Base):
    def test_save(self):
        self.check(1)

    def helper(self, unused):  # dead: helper, unused
        return 1


class PlainHelper:  # dead: PlainHelper
    pass
