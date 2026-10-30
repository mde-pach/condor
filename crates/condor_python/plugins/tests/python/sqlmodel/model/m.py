from sqlmodel import Field, SQLModel


class Hero(SQLModel, table=True):
    id: int = Field(primary_key=True)
    name: str

    def helper(self):  # dead: helper
        return 1


print(Hero)
