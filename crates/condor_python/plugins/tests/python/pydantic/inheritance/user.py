from models import Base


class User(Base):
    email: str

    def unused_method(self):  # dead: unused_method
        return 1


print(User)
