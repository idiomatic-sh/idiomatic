from src.services.reminders.service import ReminderManager, prefix


def test_remind_names_the_todo() -> None:
    assert ReminderManager().remind("Buy milk") == "Reminder: Buy milk"


def test_reminders_are_under_the_todos_prefix() -> None:
    assert prefix == "/todos"
