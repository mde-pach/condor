from django.http import HttpResponse
from django.views import View
from django.views.generic import ListView


class Home(View):
    template_name = "home.html"
    http_method_names = ["get"]
    custom_option = 1  # dead: custom_option

    def get(self, request):
        return HttpResponse()

    def helper(self):  # dead: helper
        return 1


class Books(ListView):
    paginate_by = 10

    def get_queryset(self):
        return []


class NotAView:
    def post(self, request):  # dead: post, request
        return 1


print(Home, Books, NotAView)
