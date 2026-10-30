from django.db.models.signals import post_save
from django.dispatch import receiver


@receiver(post_save)
def on_save(sender, **kwargs):
    pass


def not_a_receiver(sender):  # dead: not_a_receiver, sender
    pass
