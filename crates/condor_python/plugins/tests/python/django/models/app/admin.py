from django.contrib import admin

from app.models import Author


@admin.register(Author)
class AuthorAdmin(admin.ModelAdmin):
    list_display = ["name"]
    search_fields = ("name",)
    unused_setting = True  # dead: unused_setting

    def helper(self):  # dead: helper
        return 1


class DecadeFilter(admin.SimpleListFilter):
    title = "decade"
    parameter_name = "decade"
    spare = 1  # dead: spare

    def lookups(self, request, model_admin):
        return []

    def queryset(self, request, queryset):
        return queryset

    def helper(self, x):  # dead: helper, x
        return 1


print(DecadeFilter)
