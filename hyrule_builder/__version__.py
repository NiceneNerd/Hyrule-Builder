_major = 0
_minor = 3
_patch = "2"
VERSION = f"{_major}.{_minor}.{_patch}"
USER_VERSION = f"""{_major}.{_minor}.{_patch} {
    'alpha' if _major < 1 else ''
}{
    'beta' if 'b' in _patch else ''
}"""
