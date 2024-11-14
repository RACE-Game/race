use std::collections::BTreeMap;
use std::iter::repeat;

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq, Eq)]
pub enum RandomSpec {
    ShuffledList {
        options: Vec<String>,
    },
    Lottery {
        options_and_weights: BTreeMap<String, u16>,
    },
}

impl RandomSpec {
    pub fn as_options(self) -> Vec<String> {
        match self {
            RandomSpec::ShuffledList { options } => options,
            RandomSpec::Lottery {
                options_and_weights,
            } => options_and_weights
                .into_iter()
                .flat_map(|(o, w)| repeat(o).take(w as _))
                .collect(),
        }
    }

    /// Create a deck of cards.
    /// Use A, 2-9, T, J, Q, K for kinds.
    /// Use S(spade), D(diamond), C(club), H(heart) for suits.
    pub fn deck_of_cards() -> Self {
        RandomSpec::ShuffledList {
            options: vec![
                "ha".into(),
                "h2".into(),
                "h3".into(),
                "h4".into(),
                "h5".into(),
                "h6".into(),
                "h7".into(),
                "h8".into(),
                "h9".into(),
                "ht".into(),
                "hj".into(),
                "hq".into(),
                "hk".into(),
                "sa".into(),
                "s2".into(),
                "s3".into(),
                "s4".into(),
                "s5".into(),
                "s6".into(),
                "s7".into(),
                "s8".into(),
                "s9".into(),
                "st".into(),
                "sj".into(),
                "sq".into(),
                "sk".into(),
                "da".into(),
                "d2".into(),
                "d3".into(),
                "d4".into(),
                "d5".into(),
                "d6".into(),
                "d7".into(),
                "d8".into(),
                "d9".into(),
                "dt".into(),
                "dj".into(),
                "dq".into(),
                "dk".into(),
                "ca".into(),
                "c2".into(),
                "c3".into(),
                "c4".into(),
                "c5".into(),
                "c6".into(),
                "c7".into(),
                "c8".into(),
                "c9".into(),
                "ct".into(),
                "cj".into(),
                "cq".into(),
                "ck".into(),
            ],
        }
    }

    pub fn shuffled_list(options: Vec<String>) -> Self {
        RandomSpec::ShuffledList { options }
    }

    pub fn lottery(options_and_weights: BTreeMap<String, u16>) -> Self {
        RandomSpec::Lottery {
            options_and_weights,
        }
    }
}
