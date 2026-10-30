# planted: Consistent file names
"""The tags service lives in tags_service.py, not in the service.py every service holds."""

__all__ = ["TagsService"]


class TagsService:
    def tag(self, title: str, tag: str) -> str:
        return f"{title} #{tag}"
