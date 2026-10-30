import click


@click.group()
def cli():
    pass


@cli.command()
@click.option("--name")
def greet(name):
    click.echo(name)


@click.command()
def standalone():
    pass


def helper(value):  # dead: helper, value
    pass
