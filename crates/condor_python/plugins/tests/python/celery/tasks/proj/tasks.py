from celery import Celery, shared_task

app = Celery("proj")


@app.task
def add(x, y):
    return x + y


@shared_task(bind=True)
def notify(self):
    return self


def plain(z):  # dead: plain
    return z
