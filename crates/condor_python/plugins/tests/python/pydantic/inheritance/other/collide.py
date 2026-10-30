class Base:
    pass


class NotModel(Base):
    should_report: int  # dead: should_report


print(NotModel)
