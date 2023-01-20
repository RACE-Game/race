// use serde::{Deserialize, Serialize};
// use race_core::error::{Error, Result};
use std::cmp::Ordering;
use std::collections::HashMap;

pub fn create_cards<'a>(
    community_cards: &[&'a str; 5],
    hole_cards: &[&'a str; 2]
) -> Vec<&'a str> {
    let mut cards: Vec<&str> = Vec::new();
    cards.extend_from_slice(community_cards);
    cards.extend_from_slice(hole_cards);
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

// Cards should be sorted (strong to weak) first
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

pub fn flush_cards<'a>(cards: &Vec<&'a str>) -> Vec<&'a str> {
    let mut suit_freq: Vec<(&str, u8)> = vec![
        ("c", 0),
        ("d", 0),
        ("h", 0),
        ("s", 0)
    ];

    let suits = get_suits(cards);

    for suit in suits {
        if suit == "c" {suit_freq[0].1 += 1;}
        else if suit == "d" {suit_freq[1].1 += 1;}
        else if suit == "h" {suit_freq[2].1 += 1;}
        else {suit_freq[3].1 += 1;}
    }

    for freq in suit_freq {
        if freq.1 >= 5 {
            // Only if cards: Vec<&'a str> can be iterated but Iterator not impl-ed for &str
        }
    }

    vec![]
}

pub fn evaluate_cards(cards: Vec<&str>, kinds: Vec<&str>) -> u8 {
    let flush_cards = get_flush_cards(&cards);

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
    fn compare_hands() {
        // A single card is a 2-char string: Suit-Kind
        // For example, "hq" represents Heart Queen
        let community_cards: [&str; 5] = ["sa", "c2", "c7", "h6", "d5"];
        let hand1: [&str; 2] = ["ca", "c4"]; // pair A
        let hand2: [&str; 2] = ["d4", "h9"]; // High Card A


        let mut cards = create_cards(&community_cards, &hand1);
        // cards.extend_from_slice(&community_cards);
        // cards.extend_from_slice(&hand1);
        cards.sort_by(|c1, c2| compare_kinds(c1, c2));

        let sorted_kinds = get_sorted_kinds(&cards);

        // Test sorted cards
        assert_eq!("ca", cards[1]); // passed
        assert_eq!(vec!["sa", "ca", "c7", "h6", "d5", "c4", "c2"], cards); // passed

        // Test sorted kinds
        assert_eq!("a", sorted_kinds[0]); // passed
        assert_eq!("a", sorted_kinds[1]); // passed
        assert_eq!("2", sorted_kinds[6]); // passed

        let result: u8 = evaluate_cards(cards, sorted_kinds);
        assert_eq!(1, result); // passed

        // Test flush
        let cmt_cards: [&str; 5] = ["da", "d2", "c7", "d6", "d5"];
        let mut new_cards = create_cards(&cmt_cards, &hand2);
        cards.sort_by(|c1, c2| compare_kinds(c1, c2));

        let flush_cards = get_flush_cards(&new_cards);
        assert_eq!(5, flush_cards.len());


    }
}
