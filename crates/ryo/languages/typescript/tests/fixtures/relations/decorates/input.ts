@sealed class C {}
class Request { method(@Body() dto: Dto) {} }
@Route("/x", { method: "get" })
class Handler {}
@first @second class Twice {}
@Comp(1) export class Exported {}
@dec<T>() class Generic {}
@a.b() class Member {}
@dec /* ( */ ("arg", 2) class Commented {}
@dec("a" /* ( */) class Inner {}
