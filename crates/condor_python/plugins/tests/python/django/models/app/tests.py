from django.test import LiveServerTestCase, TestCase


class ModelTests(TestCase):
    def test_create(self):
        pass


class ServerViews(LiveServerTestCase):
    @classmethod
    def setUpClass(cls):
        super(ServerViews, cls).setUpClass()


class Helper:  # dead: Helper
    pass
