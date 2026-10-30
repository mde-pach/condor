from django import template

register = template.Library()


@register.filter
def upper(value):
    return value


@register.simple_tag
def now():
    return 1


def unregistered(value):  # dead: unregistered
    return value
