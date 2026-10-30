from flask import Blueprint, Flask

app = Flask(__name__)
bp = Blueprint("bp", __name__)


@app.route("/")
def index():
    return ""


@bp.get("/items")
def items():
    return ""


@app.before_request
def before():
    pass


@app.errorhandler(404)
def not_found(error):
    return error


@app.cli.command("init")
def init_db():
    pass


def helper():  # dead: helper
    pass
