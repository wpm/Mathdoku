use crate::mdk::domino_table::DominoTable;
use crate::mdk::mdd::Mdd;
use crate::mdk::shape::{Cell, Domino, Polyomino};
use crate::mdk::{Target, N};
use crate::mdk::operation::{Commutative, NonCommutative};

pub enum Cage {
    Given {
        cell: Cell,
        value: N,
    },
    Domino {
        domino: Domino,
        operation: NonCommutative,
        target: Target,
        memo: DominoTable,
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
    DominoTable(DominoTable),
    Given(N),
}
