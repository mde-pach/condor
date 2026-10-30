from fastapi import APIRouter, Depends, FastAPI

app = FastAPI()
router = APIRouter()


def get_db():
    return 1


@app.get("/")
def index(db=Depends(get_db)):
    return db


@router.post("/items")
async def create(item: dict):
    return item


@app.on_event("startup")
def startup():
    pass


@app.middleware("http")
async def add_header(request, call_next):
    return await call_next(request)


@app.exception_handler(ValueError)
def on_error(request, exc):
    return exc


def helper():  # dead: helper
    pass


app.include_router(router)
