class Router:
    def route(self, path):  # dead: path
        return lambda f: f


router = Router()


@router.route("/")  # dead: local_route
def local_route():
    pass
