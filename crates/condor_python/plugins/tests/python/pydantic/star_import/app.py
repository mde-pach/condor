from lib import *


class User(Model):
    email: str


class Plain(object):
    attr: int  # dead: attr


print(User, Plain)
