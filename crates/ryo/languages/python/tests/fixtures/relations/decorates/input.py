@app.route("/", methods=["GET"])
@login_required
@functools.lru_cache(maxsize=None)
def index():
    pass


@dataclass(frozen=True)
class Point:
    x: int


@handlers[0]
@factory()()
@(lambda f: f)
def dynamic():
    pass
