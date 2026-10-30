from django.db import models


class Author(models.Model):
    name = models.CharField(max_length=100)
    objects = models.Manager()
    birth: int = 0  # dead: birth
    STATUS_DRAFT = "draft"  # dead: STATUS_DRAFT

    class Meta:
        ordering = ["name"]
        verbose_name = "author"

    def save(self, *args, **kwargs):
        super().save(*args, **kwargs)

    def get_absolute_url(self):
        return "/"

    def helper(self):  # dead: helper
        return 1


class Book(Author):
    title = models.CharField(max_length=10)


class Plain:
    CONSTANT = 1  # dead: CONSTANT

    def get_absolute_url(self):  # dead: get_absolute_url
        return "/"


print(Book, Plain)
