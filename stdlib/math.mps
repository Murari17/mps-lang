// MPS Standard Library: math
// Mathematical utility functions

fn abs<T>(x: T) -> T:
    if x < (x - x):
        return -x
    return x

fn max<T>(a: T, b: T) -> T:
    if a > b:
        return a
    return b

fn min<T>(a: T, b: T) -> T:
    if a < b:
        return a
    return b

fn clamp<T>(value: T, lo: T, hi: T) -> T:
    if value < lo:
        return lo
    if value > hi:
        return hi
    return value

fn pow<T>(base: T, exp: int) -> T:
    if exp == 0:
        return base / base
    let result: T = base
    let i: int = 1
    while i < exp:
        result = result * base
        i = i + 1
    return result

fn factorial(n: int) -> int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)

fn gcd(a: int, b: int) -> int:
    while b != 0:
        let temp: int = b
        b = a % b
        a = temp
    return a

fn lcm(a: int, b: int) -> int:
    return (a * b) / gcd(a, b)

fn is_even(n: int) -> bool:
    return n % 2 == 0

fn is_odd(n: int) -> bool:
    return n % 2 != 0
