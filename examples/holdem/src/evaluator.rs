// use serde::{Deserialize, Serialize};
// use race_core::error::{Error, Result};
use std::cmp::Ordering;
use std::collections::HashMap;

// ============================================================
// Cards contains 7 cards: community cards(5) + hole cards(2)
// A hand (or picks) is the best 5 cards out of 7
// Cards should be sorted by kinds before being compared
// ============================================================

pub fn create_cards<'a>(
    community_cards: &[&'a str; 5],
    hole_cards: &[&'a str; 2]
) -> Vec<&'a str> {
    let mut cards: Vec<&str> = Vec::new();
    cards.extend_from_slice(community_cards);
    cards.extend_from_slice(hole_cards);
    cards
}

pub fn validate_cards(cards: &Vec<&str>) -> bool {
    if cards.len() == 7 { true }
    else { false }
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

// ============================================================
// All the fns below will assume that `cards' have been sorted
// using the fns above, in the order of high to low
// ============================================================
pub fn get_sorted_kinds<'a>(cards: &Vec<&'a str>) -> Vec<&'a str> {
    let mut sorted_kinds: Vec<&str> = Vec::new();
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

pub fn get_flush_cards<'a>(cards: &Vec<&'a str>) -> Vec<&'a str> {
    let mut groups: HashMap<&'a str, Vec<&'a str>> = HashMap::new();

    for card in cards {
        let (suit, _) = card.split_at(1);
        groups.entry(suit)
            .and_modify(|grp| grp.push(card))
            .or_insert(vec![card]);
    }

    for (_, val) in groups.iter() {
        if val.len() >= 5 {
            return val.clone();
        }
    }

    vec![]
}

// or [[u8;5]; 10] but array has no iter() method
fn all_straights() -> Vec<Vec<u8>> {
    vec![
        vec![14, 13, 12, 11, 10], vec![13, 12, 11, 10, 9],
        vec![12, 11, 10, 9, 8], vec![11, 10, 9, 8, 7],
        vec![10, 9, 8, 7, 6], vec![9, 8, 7, 6, 5],
        vec![8, 7, 6, 5, 4], vec![7, 6, 5, 4, 3],
        vec![6, 5, 4, 3, 2], vec![5, 4, 3, 2, 14],
    ]
}

// Check if a set of cards (7) contains straight(s) and if so return them:
// Given the sorted cards, a straight can appear in 3 places:
// first 5, middle 5, or last 5
pub fn find_straights<'a>(cards: &Vec<&'a str>) -> (bool, Vec<Vec<&'a str>>) {
    // 5 high straight is the only special case where
    // Ace ("?a"/14) need to be moved to last before any other operation
    let mut cards_to_kinds: Vec<u8> = cards.iter()
        .map(|&c| kind_to_order(c))
        .collect();
    if cards_to_kinds[0] == 14 && cards_to_kinds[1] != 13 {
        // Move Ace (14) to the last
        let ace: u8 = cards_to_kinds.remove(0);
        cards_to_kinds.push(ace);
    }

    let straights = all_straights();
    let mut result: Vec<Vec<&str>> = vec![];

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
                    // A high straight or cards got re-ordred due to Ace presence
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

    if result.len() >= 1 {
        (true, result)
    } else {
        (false, result)
    }
}

pub fn evaluate_cards(cards: Vec<&str>, kinds: Vec<&str>) -> u8 {
    let flush_cards = get_flush_cards(&cards);
    let (has_straights, straights) = find_straights(&cards);
    // royal flush

    // straight flush

    // four of a kind
    if kinds[0] == kinds[1] && kinds[0] == kinds[2] && kinds[0] == kinds[3] {
        7
    }
    // full house

    // flush
    else if flush_cards.len() == 5 {
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
    fn sorting_cards() {
        // A single card is a 2-char string: Suit-Kind
        // For example, "hq" represents Heart Queen
        let community_cards: [&str; 5] = ["sa", "c2", "c7", "h6", "d5"];
        let hand1: [&str; 2] = ["ca", "c4"]; // pair A

        let mut cards = create_cards(&community_cards, &hand1);
        cards.sort_by(|c1, c2| compare_kinds(c1, c2));

        // Test sorted cards
        assert_eq!("ca", cards[1]); // passed
        assert_eq!(vec!["sa", "ca", "c7", "h6", "d5", "c4", "c2"], cards); // passed

        // Test sorted kinds
        let sorted_kinds = get_sorted_kinds(&cards);
        assert_eq!("a", sorted_kinds[0]); // passed
        assert_eq!("a", sorted_kinds[1]); // passed
        assert_eq!("2", sorted_kinds[6]); // passed

        let result: u8 = evaluate_cards(cards, sorted_kinds);
        assert_eq!(1, result); // passed
    }

    #[test]
    fn test_flush() {
        // Test flush
        let hand2: [&str; 2] = ["d4", "h9"]; // High Card A
        let cmt_cards: [&str; 5] = ["da", "d2", "c7", "d6", "d5"];
        let mut new_cards = create_cards(&cmt_cards, &hand2);
        new_cards.sort_by(|c1, c2| compare_kinds(c1, c2));

        let flush_cards = get_flush_cards(&new_cards);
        assert_eq!(5, flush_cards.len()); // passed
        assert_eq!(vec!["da", "d6", "d5", "d4", "d2"], flush_cards); // passed
    }

    #[test]
    fn test_straights() {
        // Test one normal straight: [9,8,7,6,5]
        let hole_cards1: [&str; 2] = ["s5", "h6"];
        let cmt_cards1: [&str; 5] = ["ca", "d2", "c7", "d8", "d9"];
        let mut new_cards1 = create_cards(&cmt_cards1, &hole_cards1);
        new_cards1.sort_by(|c1, c2| compare_kinds(c1, c2));

        let (has_straights1, straights1) = find_straights(&new_cards1);

        assert!(has_straights1); // passed
        assert_eq!(1, straights1.len()); // passed
        assert_eq!(vec!["d9","d8","c7","h6","s5"], straights1[0]); // passed

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

        // Test straight that has Ace: [14,13,12,11,10] or [14,5,4,3,2]
        let hole_cards3: [&str; 2] = ["sa", "hq"];
        let cmt_cards3: [&str; 5] = ["cj", "dt", "ck", "d8", "d4"];
        let mut new_cards3 = create_cards(&cmt_cards3, &hole_cards3);
        new_cards3.sort_by(|c1, c2| compare_kinds(c1, c2));

        let (has_straights3, straights3) = find_straights(&new_cards3);
        assert!(has_straights3); // passed
        assert_eq!(1, straights3.len()); // passed
        assert_eq!(vec!["sa","ck","hq","cj","dt"], straights3[0]); // passed

        let hole_cards4: [&str; 2] = ["sa", "h7"];
        let cmt_cards4: [&str; 5] = ["c5", "d3", "c2", "s6", "d4"];
        let mut new_cards4 = create_cards(&cmt_cards4, &hole_cards4);
        new_cards4.sort_by(|c1, c2| compare_kinds(c1, c2));

        let (has_straights4, straights4) = find_straights(&new_cards4);
        assert!(has_straights4); // passed
        assert_eq!(3, straights4.len()); // passed
        assert_eq!(vec!["h7","s6","c5","d4","d3"], straights4[0]); // passed
        assert_eq!(vec!["s6","c5","d4","d3","c2"], straights4[1]); // passed
        assert_eq!(vec!["c5","d4","d3","c2","sa"], straights4[2]); // passed

    }
}
