// MPS Standard Library: strings
// String utility functions

fn repeat(s: string, times: int) -> string:
    let result: string = ""
    let i: int = 0
    while i < times:
        result = result + s
        i = i + 1
    return result

fn starts_with(s: string, prefix: string) -> bool:
    if prefix.length() > s.length():
        return false
    let i: int = 0
    while i < prefix.length():
        if s[i] != prefix[i]:
            return false
        i = i + 1
    return true

fn ends_with(s: string, suffix: string) -> bool:
    if suffix.length() > s.length():
        return false
    let offset: int = s.length() - suffix.length()
    let i: int = 0
    while i < suffix.length():
        if s[offset + i] != suffix[i]:
            return false
        i = i + 1
    return true

fn is_empty(s: string) -> bool:
    return s.length() == 0

fn reverse(s: string) -> string:
    let result: string = ""
    let i: int = s.length() - 1
    while i >= 0:
        result = result + s[i]
        i = i - 1
    return result

fn pad_left(s: string, total_len: int, pad_char: string) -> string:
    let result: string = s
    while result.length() < total_len:
        result = pad_char + result
    return result

fn pad_right(s: string, total_len: int, pad_char: string) -> string:
    let result: string = s
    while result.length() < total_len:
        result = result + pad_char
    return result
