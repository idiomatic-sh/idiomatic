from src.services.tags.tags_service import TagsService


def test_a_tag_follows_the_title() -> None:
    assert TagsService().tag("Buy milk", "home") == "Buy milk #home"
