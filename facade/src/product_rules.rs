pub const JOIN_XP_BONUS: u64 = 10;
pub const HAND_PARTICIPATION_XP: u64 = 2;
pub const HAND_WIN_BONUS: u64 = 3;
pub const SESSION_COMPLETION_XP: u64 = 20;
pub const MIN_HANDS_FOR_SESSION_COMPLETION: u64 = 3;
pub const MIN_SESSION_DURATION_SECONDS: u64 = 300;

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
