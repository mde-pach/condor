class Parser:
    def command(self):
        return lambda f: f


parser = Parser()


@parser.command()  # dead: not_click
def not_click():
    pass
