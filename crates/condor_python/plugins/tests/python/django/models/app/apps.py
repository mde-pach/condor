from django.apps import AppConfig


class LibraryConfig(AppConfig):
    name = "app"
    default_auto_field = "django.db.models.BigAutoField"
    unused = 1  # dead: unused

    def ready(self):
        pass
