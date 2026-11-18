import unittest

import forever_trainer_spells as trainer


def tables(abilities, levels):
    return {
        "SkillLine": [{"ID": "38", "CategoryID": "7"}, {"ID": "78", "CategoryID": "7"},
                      {"ID": "999", "CategoryID": "9"}],
        "SkillLineAbility": [
            {"SkillLine": line, "Spell": str(spell), "AcquireMethod": acquire, "ClassMask": mask}
            for line, spell, acquire, mask in abilities
        ],
        "SpellName": [{"ID": "1329283", "Name_lang": "Shank"},
                      {"ID": "1329549", "Name_lang": "Shank"},
                      {"ID": "401977", "Name_lang": "Shadowfiend"},
                      {"ID": "5000", "Name_lang": "Rune"},
                      {"ID": "6000", "Name_lang": "Effect"}],
        "SpellLevels": [{"SpellID": str(spell), "DifficultyID": "0", "SpellLevel": str(level),
                         "BaseLevel": "0"} for spell, level in levels.items()],
        "Talent": [{**{f"SpellRank_{rank}": "0" for rank in range(9)}, "SpellRank_0": "7000"}],
        "TraitDefinition": [{"SpellID": "7001", "VisibleSpellID": "0", "OverridesSpellID": "0"}],
    }


class GenerateTest(unittest.TestCase):
    def test_an_observed_rank_brings_its_family(self):
        rows = trainer.generate(
            tables([("38", 1329283, "0", "8"), ("38", 1329549, "0", "8")],
                   {1329283: 24, 1329549: 44}),
            [1329283],
            unlisted={},
        )
        self.assertEqual(rows, [
            {"id": 1329283, "family": 1329283, "level": 24},
            {"id": 1329549, "family": 1329283, "level": 44},
        ])

    def test_unobserved_triggered_and_non_class_rows_stay_out(self):
        rows = trainer.generate(
            tables([("38", 5000, "0", "8"), ("38", 6000, "3", "8"), ("999", 1329549, "0", "8"),
                    ("78", 401977, "0", "16")], {}),
            [6000, 1329549],
        )
        self.assertEqual(rows, [{"id": 401977, "family": 401977, "level": 0}])

    def test_a_talent_is_left_to_the_talent_string(self):
        rows = trainer.generate(
            tables([("38", 7000, "0", "8"), ("38", 7001, "0", "8")], {}), [7000, 7001],
            unlisted={},
        )
        self.assertEqual(rows, [])

    def test_an_unlisted_spell_needs_a_trainer_row(self):
        with self.assertRaises(ValueError):
            trainer.generate(tables([], {}), [])


if __name__ == "__main__":
    unittest.main()
