_MAJOR = 0
_MINOR = 6
_PATCH = "0"

VERSION = f"{_MAJOR}.{_MINOR}.{_PATCH}"
USER_VERSION = f"""{_MAJOR}.{_MINOR}.{_PATCH} {
    'alpha' if _MAJOR < 1 else ''
}{
    'beta' if 'b' in _PATCH else ''
}"""
