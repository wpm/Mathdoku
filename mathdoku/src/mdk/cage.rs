use crate::mdk::mdd::Mdd;
use crate::mdk::operation::Commutative;
use crate::mdk::shape::{Cell, Polyomino};
use crate::mdk::{N, Target};

pub enum Cage {
    Given {
        cell: Cell,
        value: N,
    },
    Polyonimo {
        polyomino: Polyomino,
        operation: Commutative,
        target: Target,
        memo: Mdd,
    },
}

pub enum Memo {
    Mdd(Mdd),
    Given(N),
}
