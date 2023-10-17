// This file also implements folding utils like shape. Should split into parts later.

use std::iter::repeat;

use ff::PrimeField;
use halo2curves::CurveExt;

use crate::{utils::field_precomp::FieldUtils, commitment::CtS};


pub trait Shape : Clone + Eq {}
pub trait IntoShape {
    type TargetShape : Shape;

    fn shape_of(&self) -> Self::TargetShape;
}
pub trait Relaxed : IntoShape {}

pub trait Strict {
    type RelaxationTarget : Relaxed;

    fn relax(&self) -> Self::RelaxationTarget;
}
pub trait Strategy : Strict {
    fn shape_of(&self) -> <<T as Strict>::RelaxationTarget as IntoShape>::TargetShape;
}


impl<T:Strict+DefaultStrategy> IntoShape for T {
    type TargetShape = <<T as Strict>::RelaxationTarget as IntoShape>::TargetShape;

    fn shape_of(&self) -> Self::TargetShape {
        self.relax().shape_of()
    }
}

pub trait VerifierView {
    type ToShape : Shape;
    type ToRelaxed : RelaxedVerifierView;

    fn shape_of(&self) -> Self::ToShape;

    fn is_of_shape(&self, shape : &Self::ToShape) -> bool {
        self.shape_of() == *shape
    }

    fn relax(self) -> Self::ToRelaxed;
}

pub trait RelaxedVerifierView {
    type ToShape: Shape;
    
    fn shape_of(&self) -> Self::ToShape;

    fn is_of_shape(&self, shape : &Self::ToShape) -> bool {
        self.shape_of() == *shape
    }
}

impl<T: RelaxedVerifierView> VerifierView for T {
    type ToShape = <Self as RelaxedVerifierView>::ToShape;
    type ToRelaxed = Self;

    fn shape_of(&self) -> Self::ToShape {
        self.shape_of()
    }

    fn is_of_shape(&self, shape : &Self::ToShape) -> bool {
        self.is_of_shape(shape)
    }

    fn relax(self) -> Self::ToRelaxed {
        self
    }
}

pub trait ProverView {
    type ToVerifierView: VerifierView;
    type ToRelaxed: RelaxedProverView;
}

pub trait RelaxedProverView {
    
}