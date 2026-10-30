class Registry:
    def task(self, f):
        return f


registry = Registry()


@registry.task  # dead: not_a_task
def not_a_task():
    pass
