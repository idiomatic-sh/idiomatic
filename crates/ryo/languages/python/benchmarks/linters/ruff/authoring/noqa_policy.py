"""Render Ruff-local query policy from one pinned code catalog and redirect source."""
from __future__ import annotations
import json
import re
from collections import defaultdict
from pathlib import Path
from authoring import profile_config


def check_policy(policy: dict[str, object]) -> None:
    if policy.get('schema') != 'pinned-noqa-code-policy/v1' or policy.get('ruff_version') != profile_config.PINNED_RUFF_VERSION or policy.get('ruff_source_commit') != profile_config.PINNED_RUFF_SOURCE_COMMIT:
        raise ValueError('different pinned noqa policy')


def render_codes(policy: dict[str, object], rule: str) -> str:
    """The exact noqa spellings that suppress a rule: its code and every redirect source of it."""
    check_policy(policy)
    spellings = sorted({rule, *(before for before, after in policy['redirects'].items() if after == rule and re.fullmatch('[A-Z]+[0-9]+', before))})
    return '^(?:' + '|'.join(spellings) + ')$'


def render_lookup(policy: dict[str, object]) -> str:
    check_policy(policy)
    codes = set(policy['registered_codes'])
    redirects = policy['redirects']
    if any(not re.fullmatch('[A-Z]+[0-9]+', code) for code in codes):
        raise ValueError('invalid registered noqa code')
    plain = sorted(codes - redirects.keys())
    pattern = '^(?:' + '|'.join(plain) + ')$'
    lines = [f'let catalog_plain = select id, file, code, code as canonical, token_span from tokens where matches(code, "{pattern}");']
    groups = defaultdict(list)
    for before, after in redirects.items():
        if after in codes and re.fullmatch('[A-Z]+[0-9]+', before):
            groups[after].append(before)
    names = ['catalog_plain']
    for index, (target, aliases) in enumerate(sorted(groups.items())):
        pattern = '^(?:' + '|'.join(sorted(aliases)) + ')$'
        lines.extend([
            f'let catalog_alias_{index} = select id, file, code, token_span from tokens where matches(code, "{pattern}");',
            f'let catalog_target_{index} = extend(from=catalog_alias_{index}, values={{canonical: "{target}"}});',
            f'let catalog_branch_{index} = select id, file, code, canonical, token_span from catalog_target_{index};',
        ])
        names.append(f'catalog_branch_{index}')
    # Balanced use of the existing shared union avoids repeated left-fold copying.
    index = 0
    while len(names) > 1:
        next_names = []
        for at in range(0, len(names), 2):
            if at + 1 == len(names):
                next_names.append(names[at])
                continue
            name = f'catalog_union_{index}'
            lines.append(f'let {name} = union(left={names[at]}, right={names[at + 1]});')
            next_names.append(name)
            index += 1
        names = next_names
    lines.append(f'let known_tokens = select id, file, code, canonical, token_span from {names[0]};')
    return '\n'.join(lines) + '\n'


def generate(root: Path) -> None:
    policy = json.loads((root / 'profiles/pinned-noqa-code-policy.json').read_text(encoding='utf-8'))
    query = root / 'rules/RUF100/query.ryoql'
    text = query.read_text(encoding='utf-8')
    end = text.index('let used_codes =')
    start = text.find('let catalog_plain =', 0, end)
    start = end if start == -1 else start
    query.write_text(text[:start] + render_lookup(policy) + text[end:], encoding='utf-8')
    aliases = sorted(code for code in policy['redirects'] if re.fullmatch('[A-Z]+[0-9]+', code))
    pattern = '^(?:' + '|'.join(aliases) + ')$'
    query = root / 'rules/RUF101/query.ryoql'
    text = query.read_text(encoding='utf-8')
    text, count = re.subn(r'(let redirected = [^\n]*matches\(code, ")[^"]+("\);)', lambda match: match[1] + pattern + match[2], text)
    if count != 1:
        raise ValueError('missing canonical redirected-code policy output')
    query.write_text(text, encoding='utf-8')

    # The same redirect source controls producer suppression and the consumer's code identity.
    suppression = json.loads((root / 'results/standard-suppression-policy.json').read_text(encoding='utf-8'))
    for code in suppression['rules']:
        query = root / f'rules/{code}/query.ryoql'
        text = query.read_text(encoding='utf-8')
        text, count = re.subn(r'\bcodes="[^"]*"', lambda match: 'codes="' + render_codes(policy, code) + '"', text)
        if count != 1:
            raise ValueError(f'{code} does not have one suppression call')
        query.write_text(text, encoding='utf-8')


if __name__ == '__main__':
    generate(profile_config.RUFF_DIR)
