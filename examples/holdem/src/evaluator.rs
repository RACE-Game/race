// use serde::{Deserialize, Serialize};
// use race_core::error::{Error, Result};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// Cards are consisted of 5 community cards + 2 hole cards.
/// Each card is represented with a string literal where
/// suit comes first, then kind: "ca" (Club Ace).
/// A hand (or picks) is the best 5 out of 7.
/// Cards can be sorted in two ways:
/// 1. by their kinds, for finding straights;
/// 2. by grouped kinds, for pairs, full house, or three/four of a kind.
pub fn create_cards<'a>(
    community_cards: &[&'a str; 5],
    hole_cards: &[&'a str; 2]
) -> Vec<&'a str> {
    let mut cards: Vec<&str> = Vec::with_capacity(7);
    cards.extend_from_slice(community_cards);
    cards.extend_from_slice(hole_cards);
    // cards.sort_by(|&c1, &c2| compare_kinds(c1, c2));
    cards
}

fn kind_to_order(card: &str) -> u8 {
    let (_, kind) = card.split_at(1);
    if kind == "a" { 14 }
    else if kind == "k" { 13 }
    else if kind == "q" { 12 }
    else if kind == "j" { 11 }
    else if kind == "t" { 10 }
    else { kind.parse::<u8>().unwrap() } // 2-9
}

/// Used as the fn required by the built-in sort_by method on Vec.
/// After sorting, higher card (kind) will come first in the vec.
/// Input:  "ck" "ha"
/// Output: Ordering::Less
pub fn compare_kinds(card1: &str, card2: &str) -> Ordering {
    let order1 = kind_to_order(card1);
    let order2 = kind_to_order(card2);

    if order2 > order1 { Ordering::Greater }
    else if order2 < order1 {Ordering::Less }
    else { Ordering::Equal }
}

pub fn validate_cards(cards: &Vec<&str>) -> bool {
    if cards.len() == 7 { true }
    else { false }
}

/// Sort the 7 cards first by the number of grouped kinds.
/// If two groups have equal number of cards, the higher-kind group wins:
/// Input:  ["ht", "st", "s8", "c8", "h5", "d3", "h3"]
/// Output: ["ht", "st", "s8", "c8", "d3", "h3", "h5"]
pub fn sort_grouped_cards<'a>(cards: &Vec<&'a str>) -> Vec<&'a str> {
    // Group cards by their kinds
    let cards_to_kinds: Vec<u8> = cards.iter()
        .map(|&c| kind_to_order(c))
        .collect();
    let mut groups: HashMap<u8, Vec<&str>> = HashMap::with_capacity(7);
    for (idx, kind) in cards_to_kinds.into_iter().enumerate() {
        groups.entry(kind)
            .and_modify(|grp| grp.push(cards[idx]))
            .or_insert(vec![cards[idx]]);
    }
    // Create a vec of key-value to sort
    let mut to_sort: Vec<(u8, Vec<&str>)> = groups.into_iter().collect();

    // Sort the (kind, cards) in the vec
    to_sort.sort_by(
        |(k1, c1), (k2, c2)| -> Ordering {
            if c2.len() > c1.len() { Ordering::Greater }
            else if c2.len() == c1.len() {
                if k2 > k1 { Ordering::Greater }
                else { Ordering::Less }
            }
            else { Ordering::Less }
        }
    );

    let result: Vec<&str> = to_sort.into_iter()
        .fold(Vec::with_capacity(7),
              |mut acc, (_, cs)| {
                  acc.extend_from_slice(&cs);
                  acc
              });
    result
}

// ============================================================
// Most fns below will assume that `cards' have been sorted
// using the fns above, in the order from high to low
// ============================================================

/// Used to detect the type of SameKinds: One Pair, Two Pairs, FullHouse, etc.
#[derive(Debug, PartialEq)]
pub enum Category {
    RoyalFlush,
    StraightFlush,
    FourOfAKind,
    FullHouse,
    Flush,
    Straight,
    ThreeOfAKind,
    TwoPairs,
    Pair,
    HighCard,
}

pub struct PlayerHand<'a> {
    pub category: Category,         // ranking
    pub picks: Vec<&'a str>,        // Best 5 of 7
    pub value: Vec<u8>,             // Values of Best 5 Kinds + category value
}


/// Given the vec of kind orders, tag the category order value in the first place
fn tag_value(picked: &Vec<&str>, catetogry_orderv: u8) -> Vec<u8> {
    let kind_values: Vec<u8> = picked.iter().map(|&c| kind_to_order(c)).collect();
    let mut value: Vec<u8> = vec![catetogry_orderv];
    value.extend_from_slice(&kind_values);
    value
}

/// Use the enum Category to check if there is indeed one
pub fn check_same_kinds(
    sorted_kinds: &Vec<&str>,
    category: Category
) -> bool {
    match category {
        Category::FourOfAKind => {
            if sorted_kinds[0..4].iter().all(|&c| c == sorted_kinds[0])
            { true }
            else { false }
        },
        Category::FullHouse => {
            if sorted_kinds[0..3].iter().all(|&c| c == sorted_kinds[0])
               && sorted_kinds[3..=4].iter().all(|&c| c == sorted_kinds[3])
            { true }
            else { false }
        },
        Category::ThreeOfAKind => {
            if sorted_kinds[0..3].iter().all(|&c| c == sorted_kinds[0])
            { true }
            else { false }
        },
        Category::TwoPairs => {
            if sorted_kinds[0] == sorted_kinds[1]
               && sorted_kinds[2] == sorted_kinds[3]
            { true }
            else { false }
        },
        Category::Pair => {
            if sorted_kinds[0] == sorted_kinds[1]
            { true }
            else { false }
        },
        _ => false
    }
}

/// Search for flush cards from the 7.
/// This fn accept sorted-by-kind or unsorted cards (preferable).
/// It returns sorted-by-kind cards anyway.
pub fn find_flush<'a>(cards: &Vec<&'a str>) -> (bool, Vec<&'a str>) {
    let mut groups: HashMap<&'a str, Vec<&'a str>> = HashMap::with_capacity(7);

    for card in cards {
        let (suit, _) = card.split_at(1);
        groups.entry(suit)
            .and_modify(|grp| grp.push(card))
            .or_insert(vec![card]);
    }

    for (_, mut val) in groups.into_iter() {
        if val.len() >= 5 {
            val.sort_by(|&c1, &c2| compare_kinds(c1, c2));
            return (true, val.clone());
        }
    }

    (false, vec![])
}

/// Check the cards against each straight of all straights.
/// In each loop, immediately add the first match to results.
/// To be used only inside the find_straights below.
fn match_straight<'a>(cards: Vec<&'a str>, result: &mut Vec<Vec<&'a str>>) {
    let straights: [[u8; 5]; 10] = [
        [14, 13, 12, 11, 10], [13, 12, 11, 10, 9],
        [12, 11, 10, 9, 8], [11, 10, 9, 8, 7],
        [10, 9, 8, 7, 6], [9, 8, 7, 6, 5],
        [8, 7, 6, 5, 4], [7, 6, 5, 4, 3],
        [6, 5, 4, 3, 2], [5, 4, 3, 2, 14],
    ];

    for straight in straights {
        let hit: Vec<&str> = cards.iter()
            .zip(straight.iter())
            .filter(|(&c, &k)| kind_to_order(c) == k)
            .map(|(&c, _)| c)
            .collect();

        if hit.len() == 5 {
            result.push(hit);
            break;
        }
    }
}

/// This fn accpets only sorted-by-kind cards
pub fn find_straights<'a>(cards: &Vec<&'a str>) -> (bool, Vec<Vec<&'a str>>) {
    let sorted_by_kinds: Vec<&str> = cards.iter().copied().collect();
    // In theory, 7 cards can yeild at most 4 straights?
    let mut result: Vec<Vec<&str>> = vec![];
    let cards_to_kinds: Vec<u8> = sorted_by_kinds.iter()
        .map(|&c| kind_to_order(c))
        .collect();

    // Group cards by kind to check if any 2 or more cards have the same kind
    let mut groups: HashMap<u8, Vec<&str>> = HashMap::with_capacity(7);
    for (idx, kind) in cards_to_kinds.iter().enumerate() {
        groups.entry(*kind)
            .and_modify(|grp| grp.push(sorted_by_kinds[idx]))
            .or_insert(vec![sorted_by_kinds[idx]]);
    }

    // Four of a kind or full house
    if groups.len() <= 4 {
        return (false, vec![vec!["Four of A Kind | Full House"]]);
    }
    // No card has the same kind
    else if groups.len() == 7 {
        // 5 high straight is the only special case where
        // Ace ("?a") need to be moved to last before any other operation
        let mut tmp_cards: Vec<&str> = sorted_by_kinds[..].to_vec();
        if tmp_cards[0].contains("a") && !tmp_cards[1].contains("k") {
            let ace: &str = tmp_cards.remove(0);
            tmp_cards.push(ace);
        }

        // In this case, a straight can possibly appear in 3 places:
        // first 5, middle 5, or last 5
        for start in 0..=2 {
            let hand: Vec<&str> = tmp_cards[start..=(start+4)].to_vec();
            match_straight(hand, &mut result);
        }

        if result.len() >= 1 { return (true, result); }
        else { return (false, result); }
    }
    // At least 2 cards have the same kind, len() == 5 | 6
    else {
        let same_kinds: Vec<Vec<&str>> = groups.values()
            .filter(|c| c.len() == 2 || c.len() == 3)
            .map(|c| c[..].to_vec())
            .collect();

        // A matrix (level <= 2) is needed for all possible straights
        // In theory, with_capacity(5)?
        let mut mtx: Vec<Vec<&str>> = Vec::new();
        if same_kinds.len() == 1 {
            // One same kind from 2 or 3 suits: [[h7 s7]] or [[h7 s7 d7]]
            for c in &same_kinds[0] { mtx.push(vec![c]) };
        } else if same_kinds.len() == 2 {
            // 4 cards with each two having the same kinds
            // [[c7, h7], [h6, s6]] => [[c7, h6], [c7, s6], [h7, h6], [h7, s6]]
            for k1 in &same_kinds[0] {
                for k2 in &same_kinds[1] {
                    mtx.push(vec![k1, k2]);
                }
            }
        }

        // Cards that each with a different kind
        let other_kinds: Vec<&str> = groups.values()
            .filter(|c| c.len() == 1)
            .map(|c| c[0])
            .collect();

        // Add them back to each ele in matrix
        for ele in &mut mtx {
            // each ele.len() == 6 | 5
            ele.extend_from_slice(&other_kinds);
            ele.sort_by(|&c1, &c2| compare_kinds(c1, c2));
            // Move Ace ("_a") to last if not A high straight
            if ele[0].contains("a") && !ele[1].contains("k") {
                let ace: &str = ele.remove(0);
                ele.push(ace);
            }
        }
        // Start matching straight(s)
        for ele in mtx {
            if ele.len() == 6 {
                for start in 0..=1 {
                    let hand: Vec<&str> = ele[start..=(start+4)].to_vec();
                    match_straight(hand, &mut result);
                }
            }
            else if ele.len() == 5 {
                match_straight(ele, &mut result);
            }
        }

        if result.len() >= 1 { return (true, result); }
        else { return (false, result); }
    }
}

/// This fn accepts either sorted-by-kind or unsorted cards (preferable).
/// It returns sorted-by-kind cards anyway.
pub fn find_royal_flush<'a>(cards: &Vec<&'a str>) -> (bool, Vec<&'a str>) {
    let royal_flush: [[&str; 5]; 4] = [
        ["ca", "ck", "cq", "cj", "ct"],
        ["da", "dk", "dq", "dj", "dt"],
        ["ha", "hk", "hq", "hj", "ht"],
        ["sa", "sk", "sq", "sj", "st"],
    ];

    let cards_set = HashSet::from(
        [cards[0], cards[1], cards[2], cards[3], cards[4], cards[5], cards[6]]
    );

    for rf in royal_flush {
        let royal_set = HashSet::from(rf);
        let mut hit: Vec<&str> = royal_set.intersection(&cards_set)
            .map(|&c| c)
            .collect();

        if hit.len() == 5 {
            hit.sort_by(|c1, c2| compare_kinds(c1, c2));
            return (true, hit);
        }
    }

    (false, vec![])
}

/// Search for straight flush from all found straights and flushes
pub fn find_straight_flush<'a>(
    flush: &Vec<&'a str>,
    straights: &Vec<Vec<&'a str>>
) -> Vec<Vec<&'a str>> {
    // [9,8,7,6,5,4,3]
    // [7,6,5,4,3,2,14]
    let flush_set: HashSet<&str> = flush.iter().map(|&c| c).collect();
    let mut result: Vec<Vec<&str>> = Vec::new(); // with_capacity(3)?

    for straight in straights {
        let straight_set: HashSet<&str> = straight.iter().map(|&c| c).collect();
        let mut hit: Vec<&str> = straight_set.intersection(&flush_set)
            .map(|&c| c)
            .collect();
        hit.sort_by(|c1, c2| compare_kinds(c1, c2));
        if hit.len() == 5 {
            // Simply move A to the end
            if hit[0].contains("a") {
                let ace: &str  = hit.remove(0);
                hit.push(ace);
            }
            result.push(hit) }
    }
    result
}

/// Compare values of two hands
pub fn compare_hands(handv1: &Vec<u8>, handv2: &Vec<u8>) -> Ordering {
    let result: Vec<i8> = handv1.iter()
        .zip(handv2.iter())
        .map(|(v1, v2)| -> i8 {
            if v1 > v2 { 1 }
            else if v1 < v2 { -1 }
            else { 0 }
        })
        .filter(|&r| r != 0)
        .collect();

    if result[0] == 1 { Ordering::Greater }
    else { Ordering::Less }

}

/// This fn accpets unsorted cards.
pub fn evaluate_cards(cards: Vec<&str>) -> PlayerHand {
    let sorted_by_group: Vec<&str> = sort_grouped_cards(&cards);
    let sorted_kinds: Vec<&str> = sorted_by_group.iter()
        .map(|&c| -> &str { let (_, k) = c.split_at(1); k })
        .collect();
    let (has_royal, rflush) = find_royal_flush(&cards);
    let (has_flush, flush_cards) = find_flush(&cards);
    let (has_straights, straights) = find_straights(&cards);

    // TODO: return the picked cards (best 5 out of 7)
    // royal flush
    if has_royal {
        let value = tag_value(&rflush, 9);
        PlayerHand {
            category: Category::RoyalFlush,
            picks: rflush,
            value,
        }
    }
    // straight flush
    else if has_flush && has_straights {
        let sflush = find_straight_flush(&flush_cards,&straights);
        let picks = sflush[0].to_vec();
        let value = tag_value(&picks, 8);
        PlayerHand {
            category: Category::StraightFlush,
            picks,
            value,
        }
    }
    // four of a kind
    else if check_same_kinds(&sorted_kinds, Category::FourOfAKind) {
        let picks = sorted_by_group[0..5].to_vec();
        let value = tag_value(&picks, 7);
        PlayerHand {
            category: Category::FourOfAKind,
            picks,
            value,
        }
    }
    // full house
    else if check_same_kinds(&sorted_kinds, Category::FullHouse) {
        let picks = sorted_by_group[0..5].to_vec();
        let value = tag_value(&picks, 6);
        PlayerHand {
            category: Category::FullHouse,
            picks,
            value,
        }
    }
    // flush
    else if has_flush {
        let picks = flush_cards[0..5].to_vec();
        let value = tag_value(&picks, 5);
        PlayerHand {
            category: Category::Flush,
            picks,
            value,
        }
    }
    // straight
    else if has_straights {
        let picks = straights[0].to_vec();
        let value = tag_value(&picks, 4);
        PlayerHand {
            category: Category::Straight,
            picks,
            value,
        }
    }
    // three of a kind
    else if check_same_kinds(&sorted_kinds, Category::ThreeOfAKind) {
        let picks = sorted_by_group[0..5].to_vec();
        let value = tag_value(&picks, 3);
        PlayerHand {
            category: Category::ThreeOfAKind,
            picks,
            value,
        }
    }
    // two pairs
    else if check_same_kinds(&sorted_kinds, Category::TwoPairs) {
        let picks = sorted_by_group[0..5].to_vec();
        let value = tag_value(&picks, 2);
        PlayerHand {
            category: Category::TwoPairs,
            picks,
            value,
        }
    }
    // pair
    else if check_same_kinds(&sorted_kinds, Category::Pair) {
        let picks = sorted_by_group[0..5].to_vec();
        let value = tag_value(&picks, 1);
        PlayerHand {
            category: Category::Pair,
            picks,
            value,
        }
    }
    // high card
    else {
        let picks = sorted_by_group[0..5].to_vec();
        let value = tag_value(&picks, 0);
        PlayerHand {
            category: Category::HighCard,
            picks,
            value,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    // #[ignore]
    fn sorting_cards() {
        // A single card is a 2-char string literal: Suit-Kind
        // For example, "hq" represents Heart Queen
        let community_cards: [&str; 5] = ["sa", "c2", "c7", "h2", "d5"];
        let hand: [&str; 2] = ["ca", "c4"]; // pair A
        let mut cards = create_cards(&community_cards, &hand);
        cards.sort_by(|&c1, &c2| compare_kinds(c1, c2));
        // Test sorted cards
        assert!(validate_cards(&cards));
        assert_eq!("ca", cards[1]);
        assert_eq!(vec!["sa", "ca", "c7", "d5", "c4", "c2", "h2"], cards);

        // Test sorting cards by grouped-kinds
        let sorted_cards = sort_grouped_cards(&cards);
        assert_eq!(7, sorted_cards.len());
        assert_eq!(vec!["sa", "ca", "c2", "h2", "c7", "d5", "c4"], sorted_cards);
    }

    #[test]
    // #[ignore]
    fn test_flush() {
        // Test flush
        let hand2: [&str; 2] = ["d2", "h9"];
        let cmt_cards: [&str; 5] = ["da", "dt", "c7", "d6", "d5"];
        let new_cards = create_cards(&cmt_cards, &hand2);
        assert!(validate_cards(&new_cards));

        let (has_flush, flush_cards) = find_flush(&new_cards);
        assert!(has_flush);
        assert_eq!(5, flush_cards.len());
        assert_eq!(vec!["da", "dt", "d6", "d5", "d2"], flush_cards);
    }

    #[test]
    // #[ignore]
    fn test_straights() {
        // Test one normal straight: two _6 cards lead to 2 straights
        // ["d9", "d8", "c7", "d6", "s5"] and ["d9", "d8", "c7", "h6", "s5"]
        let hole_cards1: [&str; 2] = ["s5", "h6"];
        let cmt_cards1: [&str; 5] = ["ca", "d6", "c7", "d8", "d9"];
        let mut new_cards1 = create_cards(&cmt_cards1, &hole_cards1);
        new_cards1.sort_by(|&c1, &c2| compare_kinds(c1, c2));

        let (has_straights1, straights1) = find_straights(&new_cards1);
        assert!(has_straights1);
        assert_eq!(2, straights1.len());
        assert_eq!(vec!["d9", "d8", "c7", "d6", "s5"], straights1[0]);
        assert_eq!(vec!["d9", "d8", "c7", "h6", "s5"], straights1[1]);

        // Test three straights: [10,9,8,7,6,5,4]
        let hole_cards2: [&str; 2] = ["st", "h9"];
        let cmt_cards2: [&str; 5] = ["c6", "d5", "c7", "d8", "d4"];
        let mut new_cards2 = create_cards(&cmt_cards2, &hole_cards2);
        new_cards2.sort_by(|c1, c2| compare_kinds(c1, c2));

        let (has_straights2, straights2) = find_straights(&new_cards2);
        assert!(has_straights2);
        assert_eq!(3, straights2.len());
        assert_eq!(vec!["st","h9","d8","c7","c6"], straights2[0]);
        assert_eq!(vec!["h9","d8","c7","c6","d5"], straights2[1]);
        assert_eq!(vec!["d8","c7","c6","d5","d4"], straights2[2]);

        // Test A hight straight [14,13,12,11,10]
        let hole_cards3: [&str; 2] = ["sa", "hq"];
        let cmt_cards3: [&str; 5] = ["cj", "dt", "ck", "sk", "hk"];
        let mut new_cards3 = create_cards(&cmt_cards3, &hole_cards3);
        new_cards3.sort_by(|c1, c2| compare_kinds(c1, c2));

        let (has_straights3, straights3) = find_straights(&new_cards3);
        assert!(has_straights3);
        assert_eq!(3, straights3.len());
        assert_eq!(vec!["sa","ck","hq","cj","dt"], straights3[0]);
        assert_eq!(vec!["sa","sk","hq","cj","dt"], straights3[1]);
        assert_eq!(vec!["sa","hk","hq","cj","dt"], straights3[2]);

        // Test Five high straight [14,5,4,3,2]
        let hole_cards4: [&str; 2] = ["sa", "h7"];
        let cmt_cards4: [&str; 5] = ["c5", "d3", "c2", "ha", "d4"];
        let mut new_cards4 = create_cards(&cmt_cards4, &hole_cards4);
        new_cards4.sort_by(|c1, c2| compare_kinds(c1, c2));

        let (has_straights4, straights4) = find_straights(&new_cards4);
        assert!(has_straights4);
        assert_eq!(2, straights4.len());
        assert_eq!(vec!["c5","d4","d3", "c2", "ha"], straights4[0]);
        assert_eq!(vec!["c5","d4","d3", "c2", "sa"], straights4[1]);

        // Test Four of a kind or full house (this is by accident)
        let hole_cards5: [&str; 2] = ["sa", "h7"];
        let cmt_cards5: [&str; 5] = ["ca", "d7", "c2", "ha", "d4"];
        let mut new_cards5 = create_cards(&cmt_cards5, &hole_cards5);
        new_cards5.sort_by(|c1, c2| compare_kinds(c1, c2));

        let (has_straights5, straights5) = find_straights(&new_cards5);
        assert!(!has_straights5);
        assert_eq!(vec!["Four of A Kind | Full House"], straights5[0]);
    }

    #[test]
    // #[ignore]
    fn test_royal_flush() {
        let hole_cards: [&str; 2] = ["sa", "sq"];
        let cmt_cards: [&str; 5] = ["sk", "hk", "hj", "sj", "st"];
        let mut new_cards = create_cards(&cmt_cards, &hole_cards);
        new_cards.sort_by(|c1, c2| compare_kinds(c1, c2));

        let (has_rf, rf) = find_royal_flush(&new_cards);
        assert!(has_rf);
        assert_eq!(5, rf.len());
        assert_eq!(vec!["sa", "sk", "sq", "sj", "st"], rf);
    }

    #[test]
    // #[ignore]
    fn test_straight_flush() {
        let hole_cards: [&str; 2] = ["ha", "h5"];
        let cmt_cards: [&str; 5] = ["h7", "h6", "h2", "h3", "h4"];
        let mut new_cards = create_cards(&cmt_cards, &hole_cards);
        new_cards.sort_by(|c1, c2| compare_kinds(c1, c2));

        let (has_f, flush) = find_flush(&new_cards);
        let (has_s, straights) = find_straights(&new_cards);
        let sf = find_straight_flush(&flush, &straights);

        assert!(has_f);
        assert!(has_s);
        assert_eq!(7, flush.len());
        assert_eq!(3, straights.len());
        assert_eq!(vec!["h7", "h6", "h5", "h4", "h3"], sf[0]);
        assert_eq!(vec!["h6", "h5", "h4", "h3", "h2"], sf[1]);
        assert_eq!(vec!["h5", "h4", "h3", "h2", "ha"], sf[2]);
    }

    #[test]
    // #[ignore]
    fn test_various_pairs() {
        let hole_cards: [&str; 2] = ["ha", "h5"];
        let cmt_cards: [&str; 5] = ["d7", "c6", "s7", "c7", "st"];
        let cards = create_cards(&cmt_cards, &hole_cards);
        let sorted_by_group: Vec<&str> = sort_grouped_cards(&cards);
        let sorted_kinds: Vec<&str> = sorted_by_group.iter()
            .map(|&c| -> &str { let (_, k) = c.split_at(1); k})
            .collect();

        // assert!(check_same_kinds(&sorted_kinds, SameKind::FourOfAKind));
        // assert!(check_same_kinds(&sorted_kinds, SameKind::FullHouse));
        assert!(check_same_kinds(&sorted_kinds, Category::ThreeOfAKind));
        // assert!(check_same_kinds(&sorted_kinds, SameKind::TwoPairs));
        // assert!(check_same_kinds(&sorted_kinds, SameKind::Pair));
    }

    #[test]
    // #[ignore]
    fn test_evaluator() {
        let hole_cards: [&str; 2] = ["ha", "h5"];
        let cmt_cards: [&str; 5] = ["d7", "c6", "s7", "c7", "st"];
        let cards = create_cards(&cmt_cards, &hole_cards);

        let result: PlayerHand = evaluate_cards(cards);
        assert_eq!(result.category, Category::ThreeOfAKind);
        assert_eq!(vec!["d7", "s7", "c7", "ha", "st"], result.picks);
        assert_eq!(vec![3, 7, 7, 7, 14, 10], result.value);

    }

    #[test]
    fn test_compare_hands() {
        let hole_cards1: [&str; 2] = ["h7", "h5"]; // three of a kind
        let hole_cards2: [&str; 2] = ["s2", "d8"]; // two pairs
        let cmt_cards: [&str; 5] = ["d7", "c6", "s6", "c7", "st"];
        let cards1 = create_cards(&cmt_cards, &hole_cards1);
        let cards2 = create_cards(&cmt_cards, &hole_cards2);
        let hand1: PlayerHand = evaluate_cards(cards1);
        let hand2: PlayerHand = evaluate_cards(cards2);

        let result = compare_hands(&hand1.value, &hand2.value);

        assert_eq!(Ordering::Greater, result);
    }
}
