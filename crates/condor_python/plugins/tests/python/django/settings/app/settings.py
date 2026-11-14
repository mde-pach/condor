# DJANGO_SETTINGS_MODULE = "app.settings": Django reads every upper-case name.
import os

BASE_DIR = os.path.dirname(__file__)
TIMEOUT = 30
DEBUG: bool = False
INSTALLED_APPS = ["app"]
DATABASES = {"default": {"NAME": os.path.join(BASE_DIR, "db.sqlite3")}}
default_timeout = 5  # dead: default_timeout


def env_flag(name):
    FALLBACK = "0"  # dead: FALLBACK
    return os.environ.get(name) == "1"


class Defaults:
    RETRIES = 3  # dead: RETRIES


USE_TZ = env_flag("USE_TZ")
CACHE_DEFAULTS = Defaults
