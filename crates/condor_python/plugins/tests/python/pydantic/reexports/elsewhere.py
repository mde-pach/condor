from pydantic import BaseModel


class Base(BaseModel):
    """Same name as models.plain.Base, unrelated."""

    name: str


print(Base)
