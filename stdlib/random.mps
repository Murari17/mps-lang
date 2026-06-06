// MPS Standard Library: random
// Random number generation utilities

fn random() -> float {
    return mps_random()
}

fn randint(min: int, max: int) -> int {
    return mps_randint(min, max)
}

fn seed(s: int) -> void {
    mps_random_seed(s)
}
