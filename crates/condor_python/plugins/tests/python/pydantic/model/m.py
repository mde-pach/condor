from typing import ClassVar

import pydantic
from pydantic import BaseModel, computed_field, field_validator, model_validator


class User(BaseModel):
    name: str
    age: int = 0
    registry: ClassVar[dict] = {}  # dead: registry

    @field_validator("name")
    @classmethod
    def strip(cls, value, info):
        return value

    @model_validator(mode="after")
    def check(self):
        return self

    @computed_field
    @property
    def label(self) -> str:
        return self.name

    def helper(self, unused):  # dead: helper, unused
        return 1

    @classmethod  # dead: build
    def build(cls, raw):  # dead: raw
        return cls


class Settings(pydantic.BaseSettings):
    debug: bool = False


class Plain:
    attr: int  # dead: attr

    def strip(self, plain_value):  # dead: plain_value
        return 1


print(User, Settings, Plain().strip)
