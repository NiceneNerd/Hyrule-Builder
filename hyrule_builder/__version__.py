_major = 0
_minor = 3
_patch = "4.post1"
VERSION = f"{_major}.{_minor}.{_patch}"
USER_VERSION = f"""{_major}.{_minor}.{_patch} {
    'alpha' if _major < 1 else ''
}{
    'beta' if 'b' in _patch else ''
}"""
