use crate::plonkish_lang::{PlonkishChallenge, PlonkishWitness};
use ark_ff::Field;
use ark_poly::{Evaluations, Radix2EvaluationDomain as R2D};
use folding::{
    columns::ExtendedFoldingColumn,
    eval_leaf::EvalLeaf,
    expressions::{ExpExtension, FoldingCompatibleExprInner, FoldingExp},
    instance_witness::ExtendedWitness,
    Alphas, FoldingCompatibleExpr, FoldingConfig,
};
use kimchi::{
    self,
    circuits::{expr::Variable, gate::CurrOrNext},
    curve::KimchiCurve,
};
use kimchi_msm::columns::Column as GenericColumn;
use strum::EnumCount;

/// Minimal environment needed for evaluating constraints.
pub struct SimpleEvalEnv<Curve: KimchiCurve, const N_COL: usize, const N_FSEL: usize> {
    pub ext_witness: ExtendedWitness<Curve, PlonkishWitness<N_COL, N_FSEL, Curve::ScalarField>>,
    pub alphas: Alphas<Curve::ScalarField>,
    pub challenges: [Curve::ScalarField; PlonkishChallenge::COUNT],
    pub error_vec: Evaluations<Curve::ScalarField, R2D<Curve::ScalarField>>,
    /// The scalar `u` that is used to homogenize the polynomials
    pub u: Curve::ScalarField,
}

impl<Curve: KimchiCurve, const N_COL: usize, const N_FSEL: usize>
    SimpleEvalEnv<Curve, N_COL, N_FSEL>
{
    fn challenge(&self, challenge: PlonkishChallenge) -> Curve::ScalarField {
        match challenge {
            PlonkishChallenge::Beta => self.challenges[0],
            PlonkishChallenge::Gamma => self.challenges[1],
            PlonkishChallenge::JointCombiner => self.challenges[2],
        }
    }

    pub fn process_extended_folding_column<
        FC: FoldingConfig<Column = GenericColumn, Curve = Curve, Challenge = PlonkishChallenge>,
    >(
        &self,
        col: &ExtendedFoldingColumn<FC>,
    ) -> EvalLeaf<Curve::ScalarField> {
        use EvalLeaf::Col;
        use ExtendedFoldingColumn::*;
        match col {
                Inner(Variable { col, row }) => {
                    let wit = match row {
                        CurrOrNext::Curr => &self.ext_witness.witness,
                        CurrOrNext::Next => panic!("not implemented"),
                    };
                    // The following is possible because Index is implemented for our
                    // circuit witnesses
                    Col(&wit[*col])
                },
                WitnessExtended(i) => Col(&self.ext_witness.extended.get(i).unwrap().evals),
                Error => panic!("shouldn't happen"),
                Constant(c) => EvalLeaf::Const(*c),
                Challenge(chall) => EvalLeaf::Const(self.challenge(*chall)),
                Alpha(i) => {
                    let alpha = self.alphas.get(*i).expect("alpha not present");
                    EvalLeaf::Const(alpha)
                }
                Selector(_s) => unimplemented!("Selector not implemented for FoldingEnvironment. No selectors are supposed to be used when it is Plonkish relations."),
        }
    }

    /// Evaluates the expression in the provided side
    pub fn eval_naive_fexpr<
        'a,
        FC: FoldingConfig<Column = GenericColumn, Curve = Curve, Challenge = PlonkishChallenge>,
    >(
        &'a self,
        exp: &FoldingExp<FC>,
    ) -> EvalLeaf<'a, Curve::ScalarField> {
        use FoldingExp::*;

        match exp {
            Atom(column) => self.process_extended_folding_column(column),
            Double(e) => {
                let col = self.eval_naive_fexpr(e);
                col.map(Field::double, |f| {
                    Field::double_in_place(f);
                })
            }
            Square(e) => {
                let col = self.eval_naive_fexpr(e);
                col.map(Field::square, |f| {
                    Field::square_in_place(f);
                })
            }
            Add(e1, e2) => self.eval_naive_fexpr(e1) + self.eval_naive_fexpr(e2),
            Sub(e1, e2) => self.eval_naive_fexpr(e1) - self.eval_naive_fexpr(e2),
            Mul(e1, e2) => self.eval_naive_fexpr(e1) * self.eval_naive_fexpr(e2),
            Pow(_e, _i) => panic!("We're not supposed to use this"),
        }
    }

    /// For FoldingCompatibleExp
    pub fn eval_naive_fcompat<
        'a,
        FC: FoldingConfig<Column = GenericColumn, Curve = Curve, Challenge = PlonkishChallenge>,
    >(
        &'a self,
        exp: &FoldingCompatibleExpr<FC>,
    ) -> EvalLeaf<'a, Curve::ScalarField> where {
        use FoldingCompatibleExpr::*;

        match exp {
            Atom(column) => {
                use FoldingCompatibleExprInner::*;
                match column {
                    Cell(Variable { col, row }) => {
                        let wit = match row {
                            CurrOrNext::Curr => &self.ext_witness.witness,
                            CurrOrNext::Next => panic!("not implemented"),
                        };
                        // The following is possible because Index is implemented for our
                        // circuit witnesses
                        EvalLeaf::Col(&wit[*col])
                    }
                    Challenge(chal) => EvalLeaf::Const(self.challenge(*chal)),
                    Constant(c) => EvalLeaf::Const(*c),
                    Extensions(ext) => {
                        use ExpExtension::*;
                        match ext {
                            U => EvalLeaf::Const(self.u),
                            Error => EvalLeaf::Col(&self.error_vec.evals),
                            ExtendedWitness(i) => {
                                EvalLeaf::Col(&self.ext_witness.extended.get(i).unwrap().evals)
                            }
                            Alpha(i) => EvalLeaf::Const(self.alphas.get(*i).unwrap()),
                            Selector(_sel) => panic!("No selectors supported yet"),
                        }
                    }
                }
            }
            Double(e) => {
                let col = self.eval_naive_fcompat(e);
                col.map(Field::double, |f| {
                    Field::double_in_place(f);
                })
            }
            Square(e) => {
                let col = self.eval_naive_fcompat(e);
                col.map(Field::square, |f| {
                    Field::square_in_place(f);
                })
            }
            Add(e1, e2) => self.eval_naive_fcompat(e1) + self.eval_naive_fcompat(e2),
            Sub(e1, e2) => self.eval_naive_fcompat(e1) - self.eval_naive_fcompat(e2),
            Mul(e1, e2) => self.eval_naive_fcompat(e1) * self.eval_naive_fcompat(e2),
            Pow(_e, _i) => panic!("We're not supposed to use this"),
        }
    }
}
