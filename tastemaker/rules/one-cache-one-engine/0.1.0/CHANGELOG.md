# one-cache-one-engine 0.1.0

The first release.

A function that makes two or more calls of one path or method the entry's `kept` globs match
(`ryo_kept`, `Cache::At`, `lint`) on the same receiver or the same named cache is reported at each
of them: an engine that keeps its work rewrites the cache directory's store when its call ends, so
a second lint call on the same keeping engine, or over the same directory, in one run rewrites what
the first kept (`self.analysis.lint(..)` twice at 445fd040a6). The receiver of a method call and
the arguments of a path call are read from `call-argument`, as a name or as the field they read, so
one lint call on `self.analysis` and one on `self.counting` are not reported. Every other lint call
goes through an engine that keeps nothing.
