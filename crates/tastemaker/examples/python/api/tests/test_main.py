from fastapi.testclient import TestClient

from src.main import app, seed_demo_todos

client = TestClient(app)


def test_the_app_serves_the_todos_under_todos() -> None:
    answer = client.get("/todos")
    assert answer.status_code == 200
    assert isinstance(answer.json(), list)


def test_the_page_and_its_styles_are_served() -> None:
    assert client.get("/").status_code == 200
    assert client.get("/static/style.css").status_code == 200


def test_the_demo_todos_are_written_out_in_order() -> None:
    demo = seed_demo_todos()
    assert (len(demo), demo[0], demo[-1]) == (58, "Buy milk", "Plan dinner")
