import platform
from importlib.metadata import version

from fastapi import FastAPI

app = FastAPI()


@app.get("/")
async def root():
    return {
        "marker": "onreza-cloud-fastapi",
        "python": platform.python_version(),
        "fastapi": version("fastapi"),
    }


@app.get("/items/{item_id}")
def read_item(item_id: int, q: str | None = None):
    return {"item_id": item_id, "q": q}
