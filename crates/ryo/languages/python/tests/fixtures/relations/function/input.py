def top(a, *args, **kwargs):
    def nested():
        pass
    return nested


class Service:
    async def fetch(self):
        await self.get()

    @staticmethod
    def build() -> "Service":
        return Service()


handler = lambda event: event
