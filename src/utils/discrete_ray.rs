use ff::PrimeField;

/// A discrete ray iterator: produces `start`, `start + increment`, `start + 2 * increment`, ...
pub struct DiscreteRay<F> {
    last: F,
    increment: F,
}

impl<F> DiscreteRay<F> {
    pub fn new(start: F, increment: F) -> Self {
        Self {
            last: start,
            increment,
        }
    }
}

impl<F: PrimeField> Iterator for DiscreteRay<F> {
    type Item = F;

    fn next(&mut self) -> Option<Self::Item> {
        let new = self.last + self.increment;
        self.last = new;

        Some(new)
    }
}