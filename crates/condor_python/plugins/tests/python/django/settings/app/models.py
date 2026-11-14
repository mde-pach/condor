from django.conf import settings
from django.db import models


class Order(models.Model):
    total = models.IntegerField()


print(settings, Order)
