from m import Record

class Local:
    pass

class Stored:
    record: Record
    items: list[Record]
    local: Local | None
    other: Missing
    q: m.Record
