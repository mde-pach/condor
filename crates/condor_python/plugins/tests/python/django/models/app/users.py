from django.contrib import admin
from django.contrib.auth.models import AbstractUser

from app.models import Book


class User(AbstractUser):
    USERNAME_FIELD = "email"
    REQUIRED_FIELDS = []
    email = AbstractUser.email


class BookInline(admin.TabularInline):
    model = Book
    extra = 1
    stale = 2  # dead: stale


class Site(admin.AdminSite):
    site_header = "Library"


print(User, BookInline, Site)
