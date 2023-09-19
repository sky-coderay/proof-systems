use super::{variables::Variables, WitnessCell};
use ark_ff::Field;

/// Witness cell with constant value
pub struct ConstantCell<F: Field> {
    value: F,
}

impl<F: Field> ConstantCell<F> {
    /// Create witness cell with constant value
    pub fn create(value: F) -> Box<ConstantCell<F>> {
        Box::new(ConstantCell { value })
    }
}

impl<const N: usize, F: Field> WitnessCell<N, F> for ConstantCell<F> {
    fn value(&self, _witness: &mut [Vec<F>; N], _variables: &Variables<F>) -> F {
        self.value
    }
}
