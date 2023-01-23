// use serde::{Deserialize, Serialize};
// use race_core::error::{Error, Result};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

// ======================================================
// Cards contain community cards(5) + hole cards(2)
// A hand (or picks) is the best 5 cards out of 7
// In most cases, cards should be sorted by kinds first
// ======================================================
type Cards<'a> = Vec<&'a str>;

fn kind_to_order(card: &str) -> u8 {
    let (_, kind) = card.split_at(1);
    if kind == "a" { 14 }
    else if kind == "k" { 13 }
    else if kind == "q" { 12 }
    else if kind == "j" { 11 }
    else if kind == "t" { 10 }
    else { kind.parse::<u8>().unwrap() } // 2-9
}

// After sorting, higher card (kind) will come first
// Input: ["ca", "h7", "sa", "c2", "c4", "h6", "d5"]
// Output: ["_a", "_a", "_7", "_6", "_5", "_4", "_2"]
pub fn compare_kinds(card1: &str, card2: &str) -> Ordering {
    let order1 = kind_to_order(card1);
    let order2 = kind_to_order(card2);

    if order2 > order1 { Ordering::Greater }
    else if order2 < order1 {Ordering::Less }
    else { Ordering::Equal }
}

// Create sorted-by-kind cards
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

pub fn validate_cards(cards: &Vec<&str>) -> bool {
    if cards.len() == 7 { true }
    else { false }
}


// ============================================================
// Most fns below will assume that `cards' have been sorted
// using the fns above, in the order of high to low
// ============================================================
pub fn get_sorted_kinds<'a>(cards: &Vec<&'a str>) -> Vec<&'a str> {
    let mut sorted_kinds: Vec<&str> = Vec::with_capacity(7);
    for card in cards {
        let (_, kind) = card.split_at(1);
        sorted_kinds.push(kind);
    }

    sorted_kinds
}

// Consider merging this fn into the above
pub fn get_suits<'a>(cards: &Vec<&'a str>) -> Vec<&'a str> {
    let mut suits: Vec<&str> = Vec::new();
    for card in cards {
        let (suit, _) = card.split_at(1);
        suits.push(suit);
    }

    suits
}

pub fn find_flush_cards<'a>(cards: &Vec<&'a str>) -> (bool, Vec<&'a str>) {
    let mut groups: HashMap<&'a str, Vec<&'a str>> = HashMap::new();

    for card in cards {
        let (suit, _) = card.split_at(1);
        groups.entry(suit)
            .and_modify(|grp| grp.push(card))
            .or_insert(vec![card]);
    }

    for (_, val) in groups.iter() {
        if val.len() >= 5 {
            return (true, val.clone());
        }
    }

    (false, vec![])
}

// or [[u8;5]; 10] but array has no iter() method?
fn all_straights() -> Vec<Vec<u8>> {
    vec![
        vec![14, 13, 12, 11, 10], vec![13, 12, 11, 10, 9],
        vec![12, 11, 10, 9, 8], vec![11, 10, 9, 8, 7],
        vec![10, 9, 8, 7, 6], vec![9, 8, 7, 6, 5],
        vec![8, 7, 6, 5, 4], vec![7, 6, 5, 4, 3],
        vec![6, 5, 4, 3, 2], vec![5, 4, 3, 2, 14],
    ]
}

pub fn find_straights<'a>(cards: &Vec<&'a str>) -> (bool, Vec<Vec<&'a str>>) {

    let mut result: Vec<Vec<&str>> = vec![];
    let mut cards_to_kinds: Vec<u8> = cards.iter()
        .map(|&c| kind_to_order(c))
        .collect();

    // Group cards by kind to check if any 2 cards with the same kind
    let mut groups: HashMap<u8, Vec<&str>> = HashMap::with_capacity(7);
    for (idx, kind) in cards_to_kinds.iter().enumerate() {
        groups.entry(*kind)
            .and_modify(|grp| grp.push(cards[idx]))
            .or_insert(vec![cards[idx]]);
    }

    // No card has the same kind
    if groups.len() == 7 {
        // 5 high straight is the only special case where
        // Ace ("?a"/14) need to be moved to last before any other operation
        if cards_to_kinds[0] == 14 && cards_to_kinds[1] != 13 {
            // Move Ace (14) to the last
            let ace: u8 = cards_to_kinds.remove(0);
            cards_to_kinds.push(ace);
        }

        let straights = all_straights();

        for start in 0..=2 {
            for straight in &straights {
                let hit: Vec<u8> = cards_to_kinds[start..=(start+4)].iter()
                    .zip(straight.iter())
                    .filter(|(&k1, &k2)| k1 == k2)
                    .map(|(k1, _)| k1)
                    .copied()
                    .collect();

                if hit.len() == 5 {
                    if hit[4] == 14 || cards_to_kinds[6] == 14 {
                        // 5 high straight or cards got re-ordred due to Ace presence
                        let mut tmp_cards: Vec<&str> = cards[..].to_vec();
                        let ace: &str = tmp_cards.remove(0);
                        tmp_cards.push(ace);
                        result.push(tmp_cards[start..=(start+4)].to_vec());
                        break;
                    } else {
                        // No Ace or A high straight: A,K,Q,J,T
                        result.push(cards[start..=(start+4)].to_vec());
                        break;
                    }
                }
            }
        }

        if result.len() >= 1 { return (true, result); }
        else { return (false, result); }
    }
    // At 2 cards have the same kind
    else {
        let same_kinds: Vec<Vec<&str>> = groups.values()
            .filter(|c| c.len() == 2 || c.len() == 3)
            .map(|c| c[..].to_vec())
            .collect();

        // A matrix (level <= 2) is needed for all possible straights
        let mut mtx: Vec<Vec<&str>> = Vec::new(); // In theory, with_capacity(5) would be enough?
        if same_kinds.len() == 0 {
            // This is four of a kind
            return (false, vec![vec!["Four of a kind"]]);
        } else if same_kinds.len() == 1 {
            // Only 2 or 3 cards of the same kind: [[h7 s7]] or [[h7 s7 d7]]
            for c in &same_kinds[0] { mtx.push(vec![c]) };
        } else if same_kinds.len() == 2 {
            // 4 cards fall into 2 groups (by kinds)
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
            ele.extend_from_slice(&other_kinds); // each ele.len() == 6 | 5
            ele.sort_by(|&c1, &c2| compare_kinds(c1, c2));
            // Move Ace ("_a") to last if not A high straight
            if ele[0].contains("a") && !ele[1].contains("k") {
                let ace: &str = ele.remove(0);
                ele.push(ace);
            }
        }

        let straights = all_straights();
        // let ln: usize = mtx[0].len();
        for ele in &mtx {
            if ele.len() == 6 {
                for start in 0..=1 {
                    for straight in &straights {
                        let hit: Vec<&str> = ele[start..=(start+4)].iter()
                            .zip(straight.iter())
                            .filter(|(&c1, &k2)| kind_to_order(c1) == k2)
                            .map(|(&c, _)| c)
                            .collect();

                        // ele already is vec of cards so no need to re-order orig cards
                        if hit.len() == 5 {
                            result.push(hit);
                            break; // A slice of ele can contain ONLY  straight
                        }
                    }

                }
            }
            else if ele.len() == 5 {
                for straight in &straights {
                    let hit: Vec<&str> = ele.iter()
                        .zip(straight.iter())
                        .filter(|(&c1, &k2)| kind_to_order(c1) == k2)
                        .map(|(&c, _)| c)
                        .collect();

                    // Need to handle 5 high straight: [Ace, 5, 4, 3, 2] => [5, 4, 3, 2, Ace]
                    if hit.len() == 5 {
                        result.push(hit);
                        break;
                    }
                }
            }
        }

        if result.len() >= 1 { return (true, result); }
        else { return (false, result); }
    }
}

// Similar to finding straights, given the sorted cards, a royal flush can
// appear only in one places: first 5
pub fn find_royal_flush<'a>(cards: &Vec<&'a str>) -> (bool, Vec<&'a str>) {

    let royal_flush = vec![
        vec!["ca", "ck", "cq", "cj", "ct"],
        vec!["da", "dk", "dq", "dj", "dt"],
        vec!["ha", "hk", "hq", "hj", "ht"],
        vec!["sa", "sk", "sq", "sj", "st"],
    ];

    for rf in &royal_flush {
        let hit: Vec<&str> = cards[0..=4].iter()
            .zip(rf.iter())
            .filter(|(&k1, &k2)| k1 == k2)
            .map(|(k1, _)| k1)
            .copied()
            .collect();

        if hit.len() == 5 {
            return (true, hit);
        }
    }

    (false, vec![])
}

pub fn find_straight_flush<'a>(
    flush: &Vec<&'a str>,
    straights: &Vec<Vec<&'a str>>
) -> Vec<&'a str> {
    // [9,8,7,6,5,4,3]
    // [7,6,5,4,3,2,14]
    let (suit, _) = flush[0].split_at(1);

    for straight in straights {
        let result: bool = straight.iter().all(|&c| c.find(suit) == Some(0));
        if result {
            // Return the first best straight flush
            return straight.clone();
        }
    }
    // Panic?
    vec![]
}

pub fn evaluate_cards(cards: Vec<&str>, kinds: Vec<&str>) -> u8 {
    let (has_flush, flush_cards) = find_flush_cards(&cards);
    let (has_straights, straights) = find_straights(&cards);
    let (has_royal, rflush) = find_royal_flush(&cards);
    // royal flush
    if has_royal {
        9
    }
    // straight flush
    else if has_flush && has_straights {
        8
    }
    // four of a kind
    else if kinds[0] == kinds[1] && kinds[0] == kinds[2] && kinds[0] == kinds[3] {
        7
    }
    // full house

    // flush
    else if has_flush {
        5
    }
    // straight
    else if has_straights {
        4
    }
    // three of a kind
    else if kinds[0] == kinds[1] && kinds[0] == kinds[2] {
        3
    }
    // two pairs
    else if kinds[0] == kinds[1] && kinds[2] == kinds[3] {
        2
    }
    // pair
    else if kinds[0] == kinds[1] {
        1
    }
    // high card
    else {
        0
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn sorting_cards() {
        // A single card is a 2-char string: Suit-Kind
        // For example, "hq" represents Heart Queen
        let community_cards: [&str; 5] = ["sa", "c2", "c7", "h6", "d5"];
        let hand: [&str; 2] = ["ca", "c4"]; // pair A
        let mut cards = create_cards(&community_cards, &hand);
        cards.sort_by(|&c1, &c2| compare_kinds(c1, c2));
        // Test sorted cards
        assert!(validate_cards(&cards));
        assert_eq!("ca", cards[1]); // passed
        assert_eq!(vec!["sa", "ca", "c7", "h6", "d5", "c4", "c2"], cards); // passed

        // Test sorted kinds
        let sorted_kinds = get_sorted_kinds(&cards);
        assert_eq!("a", sorted_kinds[0]); // passed
        assert_eq!("a", sorted_kinds[1]); // passed
        assert_eq!("2", sorted_kinds[6]); // passed

        // let result: u8 = evaluate_cards(cards, sorted_kinds);
        // assert_eq!(1, result); // passed
    }

    #[test]
    #[ignore]
    fn test_flush() {
        // Test flush
        let hand2: [&str; 2] = ["d4", "h9"]; // High Card A
        let cmt_cards: [&str; 5] = ["da", "d2", "c7", "d6", "d5"];
        let new_cards = create_cards(&cmt_cards, &hand2);
        assert!(validate_cards(&new_cards));

        let (has_flush, flush_cards) = find_flush_cards(&new_cards);
        assert!(has_flush);
        assert_eq!(5, flush_cards.len()); // passed
        assert_eq!(vec!["da", "d6", "d5", "d4", "d2"], flush_cards); // passed
    }

    #[test]
    fn test_straights() {
        // Test one normal straight: two _6 cards lead to 2 straights
        // ["d9", "d8", "c7", "d6", "s5"] and ["d9", "d8", "c7", "h6", "s5"]
        let hole_cards1: [&str; 2] = ["s5", "h6"];
        let cmt_cards1: [&str; 5] = ["ca", "d6", "c7", "d8", "d9"];
        let mut new_cards1 = create_cards(&cmt_cards1, &hole_cards1);
        new_cards1.sort_by(|&c1, &c2| compare_kinds(c1, c2));

        let (has_straights1, straights1) = find_straights(&new_cards1);
        assert!(has_straights1); // passed
        assert_eq!(2, straights1.len()); // passed
        assert_eq!(vec!["d9", "d8", "c7", "d6", "s5"], straights1[0]); // passed
        assert_eq!(vec!["d9", "d8", "c7", "h6", "s5"], straights1[1]); // passed

        // Test three straights: [10,9,8,7,6,5,4]
        let hole_cards2: [&str; 2] = ["st", "h9"];
        let cmt_cards2: [&str; 5] = ["c6", "d5", "c7", "d8", "d4"];
        let mut new_cards2 = create_cards(&cmt_cards2, &hole_cards2);
        new_cards2.sort_by(|c1, c2| compare_kinds(c1, c2));

        let (has_straights2, straights2) = find_straights(&new_cards2);
        assert!(has_straights2); // passed
        assert_eq!(3, straights2.len()); // passed
        assert_eq!(vec!["st","h9","d8","c7","c6"], straights2[0]); // passed
        assert_eq!(vec!["h9","d8","c7","c6","d5"], straights2[1]); // passed
        assert_eq!(vec!["d8","c7","c6","d5","d4"], straights2[2]); // passed

        // // Test straight that has Ace: [14,13,12,11,10] or [14,5,4,3,2]
        let hole_cards3: [&str; 2] = ["sa", "hq"];
        let cmt_cards3: [&str; 5] = ["cj", "dt", "ck", "sk", "hk"];
        let mut new_cards3 = create_cards(&cmt_cards3, &hole_cards3);
        new_cards3.sort_by(|c1, c2| compare_kinds(c1, c2));
        //
        let (has_straights3, straights3) = find_straights(&new_cards3);
        assert!(has_straights3); // passed
        assert_eq!(3, straights3.len()); // passed
        assert_eq!(vec!["sa","ck","hq","cj","dt"], straights3[0]); // passed
        assert_eq!(vec!["sa","sk","hq","cj","dt"], straights3[1]); // passed
        assert_eq!(vec!["sa","hk","hq","cj","dt"], straights3[2]); // passed

        //
        // let hole_cards4: [&str; 2] = ["sa", "h7"];
        // let cmt_cards4: [&str; 5] = ["c5", "d3", "c2", "s6", "d4"];
        // let new_cards4 = create_cards(&cmt_cards4, &hole_cards4);
        // // new_cards4.sort_by(|c1, c2| compare_kinds(c1, c2));
        //
        // let (has_straights4, straights4) = find_straights(&new_cards4);
        // assert!(has_straights4); // passed
        // assert_eq!(3, straights4.len()); // passed
        // assert_eq!(vec!["h7","s6","c5","d4","d3"], straights4[0]); // passed
        // assert_eq!(vec!["s6","c5","d4","d3","c2"], straights4[1]); // passed
        // assert_eq!(vec!["c5","d4","d3","c2","sa"], straights4[2]); // passed

    }

    #[test]
    #[ignore]
    fn test_royal_flush() {
        let hole_cards: [&str; 2] = ["sa", "sq"];
        let cmt_cards: [&str; 5] = ["sk", "hk", "hj", "sj", "st"];
        let mut new_cards = create_cards(&cmt_cards, &hole_cards);
        new_cards.sort_by(|c1, c2| compare_kinds(c1, c2));

        let (has_rf, rf) = find_royal_flush(&new_cards);
        assert!(has_rf);
        assert_eq!(1, rf.len());
        assert_eq!(vec!["sa", "sk", "sq", "sj", "st"], rf);
    }

    #[test]
    #[ignore]
    fn test_straight_flush() {
        let hole_cards: [&str; 2] = ["ha", "h7"];
        let cmt_cards: [&str; 5] = ["h5", "h6", "h2", "h3", "h4"];
        let mut new_cards = create_cards(&cmt_cards, &hole_cards);
        new_cards.sort_by(|c1, c2| compare_kinds(c1, c2));

        let (has_f, flush) = find_flush_cards(&new_cards);
        let (has_s, straights) = find_straights(&new_cards);
        let sf = find_straight_flush(&flush, &straights);

        assert!(has_f); // passed
        assert!(has_s); // passed
        assert_eq!(7, flush.len()); // passed
        assert_eq!(3, straights.len()); // passed
        assert_eq!(vec!["h7", "h6", "h5", "h4", "h3"], sf); // passed
    }
}
