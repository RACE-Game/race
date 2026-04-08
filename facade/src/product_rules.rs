pub const JOIN_XP_BONUS: u64 = 10;
pub const HAND_PARTICIPATION_XP: u64 = 2;
pub const HAND_WIN_BONUS: u64 = 3;
pub const SESSION_COMPLETION_XP: u64 = 20;
pub const MIN_HANDS_FOR_SESSION_COMPLETION: u64 = 3;
pub const MIN_SESSION_DURATION_SECONDS: u64 = 300;
pub const MIN_HANDS_FOR_RATING: u64 = 5;
pub const MIN_SESSION_DURATION_SECONDS_FOR_RATING: u64 = 300;
pub const MIN_OPPONENTS_FOR_RATING: u32 = 2;
pub const MIN_RATING: i32 = 100;
pub const STRONG_POSITIVE_RATING_DELTA: i32 = 25;
pub const MODERATE_POSITIVE_RATING_DELTA: i32 = 15;
pub const NEUTRAL_RATING_DELTA: i32 = 0;
pub const MODERATE_NEGATIVE_RATING_DELTA: i32 = -15;
pub const STRONG_NEGATIVE_RATING_DELTA: i32 = -25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableResultCategory {
    StrongPositive,
    ModeratePositive,
    Neutral,
    ModerateNegative,
    StrongNegative,
}

const LEVEL_THRESHOLDS: &[(u32, u64)] = &[
    (1, 0),
    (2, 100),
    (3, 250),
    (4, 450),
    (5, 700),
    (6, 1_000),
    (7, 1_350),
    (8, 1_750),
    (9, 2_200),
    (10, 2_700),
];

pub fn level_for_xp(xp: u64) -> u32 {
    let mut level = 1;
    for (candidate_level, threshold) in LEVEL_THRESHOLDS {
        if xp >= *threshold {
            level = *candidate_level;
        } else {
            return level;
        }
    }

    let extra_xp = xp.saturating_sub(2_700);
    level + (extra_xp / 600) as u32
}

pub fn rank_tier_for_level(level: u32) -> &'static str {
    match level {
        1..=2 => "Bronze I",
        3..=4 => "Bronze II",
        5..=6 => "Silver I",
        7..=8 => "Silver II",
        9..=10 => "Gold I",
        11..=12 => "Gold II",
        13..=15 => "Platinum",
        16..=20 => "Diamond",
        _ => "Hero",
    }
}

pub fn classify_table_result(entry_value: u64, ending_value: u64) -> TableResultCategory {
    if entry_value == 0 {
        return TableResultCategory::Neutral;
    }

    let baseline = entry_value as f64;
    let net = ending_value as f64 - baseline;
    let ratio = net / baseline;

    if ratio > 0.20 {
        TableResultCategory::StrongPositive
    } else if ratio > 0.0 {
        TableResultCategory::ModeratePositive
    } else if ratio < -0.20 {
        TableResultCategory::StrongNegative
    } else if ratio < 0.0 {
        TableResultCategory::ModerateNegative
    } else {
        TableResultCategory::Neutral
    }
}

pub fn rating_delta_for_result(category: TableResultCategory) -> i32 {
    match category {
        TableResultCategory::StrongPositive => STRONG_POSITIVE_RATING_DELTA,
        TableResultCategory::ModeratePositive => MODERATE_POSITIVE_RATING_DELTA,
        TableResultCategory::Neutral => NEUTRAL_RATING_DELTA,
        TableResultCategory::ModerateNegative => MODERATE_NEGATIVE_RATING_DELTA,
        TableResultCategory::StrongNegative => STRONG_NEGATIVE_RATING_DELTA,
    }
}

pub fn rank_bucket_for_rating(rating: i32) -> &'static str {
    match rating {
        i32::MIN..=999 => "Bronze",
        1000..=1199 => "Silver",
        1200..=1399 => "Gold",
        1400..=1599 => "Platinum",
        1600..=1799 => "Diamond",
        _ => "Hero",
    }
}
