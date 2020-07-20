_MAJOR = 0
_MINOR = 5
_PATCH = "7"

VERSION = f"{_MAJOR}.{_MINOR}.{_PATCH}"
USER_VERSION = f"""{_MAJOR}.{_MINOR}.{_PATCH} {
    'alpha' if _MAJOR < 1 else ''
}{
    'beta' if 'b' in _PATCH else ''
}"""
