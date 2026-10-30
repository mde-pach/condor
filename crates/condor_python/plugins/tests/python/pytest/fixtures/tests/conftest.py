import pytest


@pytest.fixture
def db():
    return 1


@pytest.fixture(name="client")
def make_client(db):
    return db


@pytest.fixture(autouse=True)
def reset():
    pass


@pytest.fixture(scope="session")
def requested_by_mark():
    return 1


@pytest.fixture  # dead: unused_fixture
def unused_fixture():
    pass


@pytest.fixture(name="renamed_unused")  # dead: renamed_unused_fn
def renamed_unused_fn():
    pass


def helper(value):  # dead: helper, value
    return 1
