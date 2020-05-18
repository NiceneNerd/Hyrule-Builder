_major = 0
_minor = 4
_patch = "0"
VERSION = f"{_major}.{_minor}.{_patch}"
USER_VERSION = f"""{_major}.{_minor}.{_patch} {
    'alpha' if _major < 1 else ''
}{
    'beta' if 'b' in _patch else ''
}"""
