class Cache:
    def get(self, key):
        return key

    def delete(self, key):
        return key


cache = Cache()


@cache.delete  # dead: dead_handler
def dead_handler():
    pass


print(cache.get)
