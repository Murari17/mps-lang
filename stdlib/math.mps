// MPS Standard Library: math
// Mathematical utility functions

fn abs(x: int) -> int {
    if x < 0 {
        return -x
    }
    return x
}

fn max(a: int, b: int) -> int {
    if a > b {
        return a
    }
    return b
}

fn min(a: int, b: int) -> int {
    if a < b {
        return a
    }
    return b
}

fn clamp(value: int, lo: int, hi: int) -> int {
    if value < lo {
        return lo
    }
    if value > hi {
        return hi
    }
    return value
}

fn pow(base: int, exp: int) -> int {
    let result: int = 1
    let i: int = 0
    while i < exp {
        result = result * base
        i = i + 1
    }
    return result
}

fn factorial(n: int) -> int {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

fn gcd(a: int, b: int) -> int {
    while b != 0 {
        let temp: int = b
        b = a % b
        a = temp
    }
    return a
}

fn lcm(a: int, b: int) -> int {
    return (a * b) / gcd(a, b)
}

fn is_even(n: int) -> bool {
    return n % 2 == 0
}

fn is_odd(n: int) -> bool {
    return n % 2 != 0
}
