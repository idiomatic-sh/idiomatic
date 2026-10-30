from fastapi.testclient import TestClient

from src.main import app

client = TestClient(app)


def test_a_todo_is_created_and_completed() -> None:
    created = client.post("/todos", json={"title": " Buy milk "}).json()
    assert created["title"] == "Buy milk"
    done = client.post(f"/todos/{created['id']}/complete").json()
    assert done["done"] is True
    assert client.post("/todos/999/complete").status_code == 404


def test_the_count_is_the_number_of_todos() -> None:
    assert client.get("/todos/count").json() == len(client.get("/todos").json())
