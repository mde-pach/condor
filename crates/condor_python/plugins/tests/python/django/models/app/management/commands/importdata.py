from django.core.management.base import BaseCommand


class Command(BaseCommand):
    help = "Import data"
    requires_system_checks = []
    leftover = 1  # dead: leftover

    def add_arguments(self, parser):
        pass

    def handle(self, *args, **options):
        pass

    def helper(self):  # dead: helper
        return 1
