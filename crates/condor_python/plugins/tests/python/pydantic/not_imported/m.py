class BaseModel:
    pass


class Model(BaseModel):
    field: int  # dead: field


def field_validator(name):  # dead: name
    return lambda f: f


class Other:
    @field_validator("x")  # dead: check
    def check(self):
        return 1


print(Model, Other)
