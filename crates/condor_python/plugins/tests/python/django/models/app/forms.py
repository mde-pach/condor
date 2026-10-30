from django import forms


class ContactForm(forms.Form):
    email = forms.EmailField()
    template_name = "contact.html"
    CHOICES = [("a", "A")]  # dead: CHOICES

    def clean_email(self):
        return self.cleaned_data["email"]

    def clean(self):
        return super().clean()

    def helper(self):  # dead: helper
        return 1


class NotAForm:
    def clean_email(self):  # dead: clean_email
        return 1


print(ContactForm, NotAForm)
