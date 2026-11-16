//! The random enchantments the Forever client rolls ("of the Sorcerer"), read under Forever's
//! rules only. The client has no ItemRandomProperties table: an item's suffix is an item bonus
//! list, whose stats are shares of the item's random property points, in ten thousandths,
//! rounded half up. Potent Boots (15171, 16 points) of the Sorcerer (12745: Intellect 7266,
//! Stamina 4106, spell power 3725) reads +12 Intellect, +7 Stamina and 6 spell power, and Dire
//! Nail (10828, 14 points) of the Sorcerer (13104, spell power 47500) reads 67 from 66.5, as
//! Wowhead's Forever tooltips of the items with those bonuses do.
//!
//! A request states the bonus list as the item's `random_suffix`: the lists start at 12688, above
//! every suffix of the reference database. `tools/forever_random_enchantments.py` writes both
//! tables from the client's ItemBonus, ItemBonusTreeNode, ItemXBonusTree, ItemNameDescription and
//! RandPropPoints tables.

use crate::data::{RANDOM_ENCHANTMENTS, RANDOM_ENCHANTMENT_ITEMS};

use super::items::RandomSuffix;
use super::stats::{Stat, Stats};
use super::Refusal;

fn row(table: &crate::data::Table, file: &str, key: i32) -> Option<serde_json::Value> {
    let text = table.row(i64::from(key))?;
    Some(serde_json::from_str(text).unwrap_or_else(|err| panic!("data/{file} row {key}: {err}")))
}

/// A share of the item's points as the tooltip states it.
fn share(points: i64, ten_thousandths: i64) -> f64 {
    ((points * ten_thousandths + 5000) / 10000) as f64
}

fn pairs<'a>(row: &'a serde_json::Value, field: &str) -> impl Iterator<Item = (Stat, i64)> + 'a {
    let field = field.to_string();
    row[field.as_str()]
        .as_array()
        .into_iter()
        .flatten()
        .map(move |pair| {
            let name = pair[0].as_str().unwrap_or_default();
            let stat = Stat::by_name(name).unwrap_or_else(|| {
                panic!("data/forever-random-enchantments.jsonl: no stat {name} ({field})")
            });
            (stat, pair[1].as_i64().unwrap_or_default())
        })
}

/// The client's random enchantment `id` on `item`, or `None` when the client has no bonus list of
/// that ID, which leaves the ID to the reference database. An item the client does not roll the
/// enchantment on is refused.
pub(crate) fn random_enchantment(item: i32, id: i32) -> Option<Result<RandomSuffix, Refusal>> {
    let enchantment = row(
        &RANDOM_ENCHANTMENTS,
        "forever-random-enchantments.jsonl",
        id,
    )?;
    let name = enchantment["name"].as_str().unwrap_or_default().to_string();
    let rolls = row(
        &RANDOM_ENCHANTMENT_ITEMS,
        "forever-random-enchantment-items.jsonl",
        item,
    )
    .filter(|rolls| {
        rolls["enchantments"].as_array().is_some_and(|pool| {
            pool.iter()
                .any(|entry| entry.as_i64() == Some(i64::from(id)))
        })
    });
    let Some(rolls) = rolls else {
        return Some(Err(Refusal::new(
            "unknown_item",
            format!("item {item} does not roll random enchantment {id} ({name})"),
        )));
    };
    let points = rolls["points"].as_i64().unwrap_or_default();
    let mut stats = Stats::default();
    for (stat, ten_thousandths) in pairs(&enchantment, "allocations") {
        stats[stat] += share(points, ten_thousandths);
    }
    for (stat, value) in pairs(&enchantment, "stats") {
        stats[stat] += value as f64;
    }
    Some(Ok(RandomSuffix { id, name, stats }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(item: i32, id: i32) -> Vec<(Stat, f64)> {
        let suffix = random_enchantment(item, id).unwrap().unwrap();
        Stat::ALL
            .into_iter()
            .filter(|stat| suffix.stats[*stat] != 0.0)
            .map(|stat| (stat, suffix.stats[stat]))
            .collect()
    }

    /// Wowhead's Forever tooltips of each item with the bonus, 16 November 2026.
    #[test]
    fn stats_are_the_tooltips() {
        use Stat::*;
        // Potent Boots of the Sorcerer: +7 Stamina, +12 Intellect, 6 damage and healing.
        assert_eq!(
            stats(15171, 12745),
            [
                (Stamina, 7.0),
                (Intellect, 12.0),
                (HealingPower, 6.0),
                (SpellDamage, 6.0)
            ]
        );
        // of the Monkey: +11 Agility, +11 Stamina.
        assert_eq!(stats(15171, 12721), [(Agility, 11.0), (Stamina, 11.0)]);
        // of Power: +32 Attack Power, melee and ranged.
        assert_eq!(
            stats(15171, 12733),
            [(AttackPower, 32.0), (RangedAttackPower, 32.0)]
        );
        // of Magic: 19 damage and healing.
        assert_eq!(
            stats(15171, 12734),
            [(HealingPower, 19.0), (SpellDamage, 19.0)]
        );
        // of Healing: healing 35, damage 12.
        assert_eq!(
            stats(15171, 12735),
            [(HealingPower, 35.0), (SpellDamage, 12.0)]
        );
        // Solstice Staff (26 points) of the Sorcerer: the two hand list, 52 damage and healing.
        assert_eq!(
            stats(15278, 12905),
            [
                (Stamina, 13.0),
                (Intellect, 20.0),
                (HealingPower, 52.0),
                (SpellDamage, 52.0)
            ]
        );
        // Holy War Sword (12 points) of the Sorcerer: the one hand list, 57.
        assert_eq!(
            stats(15221, 12906),
            [
                (Stamina, 6.0),
                (Intellect, 9.0),
                (HealingPower, 57.0),
                (SpellDamage, 57.0)
            ]
        );
        // of Defense +17, of Holy Wrath 37.
        assert_eq!(stats(15278, 12736), [(DefenseRating, 17.0)]);
        assert_eq!(stats(15278, 12759), [(HolyDamage, 37.0)]);
        // Prismatic Band (16 points) of the Champion: +7 Strength, +12 Stamina, Defense +3.
        assert_eq!(
            stats(12017, 12753),
            [(Strength, 7.0), (Stamina, 12.0), (DefenseRating, 3.0)]
        );
        // of Eluding: +5 Agility and 1% dodge. of Concentration: 6 mana per 5 sec.
        assert_eq!(stats(12017, 12739), [(Agility, 5.0), (DodgeRating, 12.0)]);
        assert_eq!(stats(12017, 12738), [(MP5, 6.0)]);
        // of Regeneration is health per 5 sec, which the sim has no stat for.
        assert_eq!(stats(12017, 12737), []);
        // Dire Nail, a rare whose row states no points: 14, so 66.5 spell power reads 67.
        assert_eq!(
            stats(10828, 13104),
            [
                (Stamina, 6.0),
                (Intellect, 9.0),
                (HealingPower, 67.0),
                (SpellDamage, 67.0)
            ]
        );
    }

    #[test]
    fn an_item_takes_only_its_own_rolls() {
        // Potent Boots do not roll a weapon list, nor a staff the one hand list, and
        // Thunderfury rolls nothing.
        assert!(random_enchantment(15171, 12906).unwrap().is_err());
        assert!(random_enchantment(15278, 12906).unwrap().is_err());
        assert!(random_enchantment(19019, 12745).unwrap().is_err());
        // A suffix of the reference database is not the client's.
        assert!(random_enchantment(15171, 611).is_none());
    }

    #[test]
    fn every_row_reads() {
        for item in RANDOM_ENCHANTMENT_ITEMS.keys() {
            let rolls = row(&RANDOM_ENCHANTMENT_ITEMS, "items", item as i32).unwrap();
            assert!(rolls["points"].as_i64().unwrap() > 0, "item {item}");
            for id in rolls["enchantments"].as_array().unwrap() {
                let id = id.as_i64().unwrap() as i32;
                assert!(
                    random_enchantment(item as i32, id).unwrap().is_ok(),
                    "{item} {id}"
                );
            }
        }
        // No bonus list shares an ID with a suffix of the reference database.
        for id in RANDOM_ENCHANTMENTS.keys() {
            assert!(crate::data::RANDOM_SUFFIXES.row(id).is_none(), "{id}");
        }
    }
}
