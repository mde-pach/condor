import pytest

pytestmark = pytest.mark.usefixtures("requested_by_mark")
UNUSED_CONSTANT = 1  # dead: UNUSED_CONSTANT


def test_client(client, tmp_path):
    pass


class TestGroup:
    def test_method(self, db):
        pass

    def helper(self, arg):  # dead: helper, arg
        pass


def make_data(size):  # dead: make_data, size
    return []
