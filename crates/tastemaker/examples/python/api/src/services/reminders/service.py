"""The reminders service."""

from src.routers.todos.routes import router  # planted: Import direction

__all__ = ["ReminderManager", "prefix"]


class ReminderManager:  # planted: Naming convention
    def remind(self, title: str) -> str:
        return f"Reminder: {title}"


prefix = router.prefix
