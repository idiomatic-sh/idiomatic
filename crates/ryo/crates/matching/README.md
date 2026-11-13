# idiomatic-sh-ryo-matching

Pure regex and glob semantics through public inbound use cases; callers own compiled storage and lifetime.

## Domains

### globbing

Text-glob algorithms preserve character, escape, alternative and directory semantics with internal compiler fallback.

### matching

Compiles and searches regexes and glob sets without acquiring execution caches or scheduling work.
