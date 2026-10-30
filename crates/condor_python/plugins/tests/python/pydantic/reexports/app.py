from models import Base, Model


class User(Model):
    email: str


class Thing(Base):
    attr: int  # dead: attr


print(User, Thing)
