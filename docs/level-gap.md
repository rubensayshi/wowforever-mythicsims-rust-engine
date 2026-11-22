# A target of any level

Status: 23 November 2026. Under Forever's rules a character fights a target from level 10 to 5
above its own. Before, the gate let through its own level to 3 above (60 to 63 at level 60).

## Why the range was 0 to 3 above

The reference's attack table has a column for a target at the character's level, 1, 2 and 3 above
it, and 2 below it (`character::unit_level_f64`). Any other level reads the 3 above column: a
level 55 dungeon mob and a level 64 boss both got the raid boss's 8% miss, 40% glancing blows and
17% spell miss. So the gate refused them (`levels::target_levels`).

What reads the gap between the two levels:

| Table | Where | Outside the columns before |
| --- | --- | --- |
| A player's miss, dodge, parry, block, glancing chance and damage, hit and crit suppression, spell miss against an enemy | `prepare/attack.rs` `new_attack_table` | the 3 above column |
| An enemy's miss, dodge, parry, block and crushing blow against a player | the same function, by the enemy's level | the 3 above column |
| An enemy's crit | `prepare/target.rs` | the 3 above column |
| Weapon skill | `prepare/weapon_skill.rs` | general above the attacker, nothing below it |
| Partial resists by level | `mechanics/damage.rs` | none under Forever's rules at any level (docs/mechanics.md), nothing to change |
| Armor | `prepare/enemy.rs`, `core/fight/melee.rs` | reads the attacker's level alone, nothing to change |
| Rage from damage taken | `prepare/rage.rs` | by the attacker's level: measured from 17 to 59, held at 21.8% below and at the notes' 40% from 63 up |
| Spell Vulnerability | `core/fight/damage.rs` | 1 / (5 x target level) a point of Spell Piercing past the resistance, fitted on a target 2 below its caster; a much lower target takes much more, which no log covers |

The prepared v1 Frostbolt kernel (`classes/mage/specs/frost.rs`) keeps its 60 to 63.

## The rules

`prepare/level_gap.rs`, by the gap between the target's level and the character's. A gap the
reference has a column for (0 to 3 above, 2 below) never reaches it, so every value those targets
read is the same number as before. Under `Rules::GoReference` nothing changes at any level.

| Table | Below the character | 4 and 5 above | Source |
| --- | --- | --- | --- |
| Miss | 5% less 0.5% a level, none from 10 below | 9%, 10% | Below: the logs, and the line the 2 below column is on (0.1% a point of defense under skill). Above: the weapon skill formula at base skill, 5% and 0.2% a point past a 10 point gap, where the 3 above column sits. The logs agree with it and cannot rule out 10% at 4 above. |
| Dodge | the same | 7%, 7.5% | Below: the logs. Above: Blizzard, 0.5% a level. |
| Parry | the same | 17%, 20% | An approximation both ways: the 2 below column's line below, 5% and 0.6% a point above, which is the 14% at 3 above and is measured nowhere else. Only an attack from the front can be parried, so a damage dealer's sim never reads it. |
| Block | 5% | 5% | The columns. |
| Glancing chance | none | 50%, 60% | The logs. |
| Glancing damage | no swing glances | 0.45 and 0.15 either side, 0.25 and 0.20 | The weapon skill formula's low and high ends, the logs at 4 above. |
| Hit suppression | none | 1% | Blizzard: the first 1% of hit against a monster more than 10 defense above. Flat, where the weapon skill formula kept adding 0.2% a point past the boss's 15. |
| Melee crit suppression | none | 5.8%, 6.8% | Blizzard: 1% a level. The 1.8% the 3 above column adds to its 3% is carried, an approximation past 3 above. |
| Spell miss | 3%, 2%, 1%, then none (the fight's own 1% floor still applies) | 28%, 39% | Blizzard's table, the logs. |
| An enemy's miss, block, parry and dodge against a player | 0.2% a level, floored at none | the same line | The line the columns are on (0.04% a point). Not checked on logs. |
| Crushing blow | none | 25%, 35% | An approximation: 2% a point of the enemy's skill over the player's defense, less 15%, which is the 15% at 3 above. |
| An enemy's crit | 5% and 0.2% a level, floored at none | 5.8%, 6% | The columns' line. |
| Weapon skill | moves nothing, as before | its formulas, from the wider gap | Unchanged. |

Not modelled:

- Crit against a lower target. Community sources add 0.2% a level (0.04% a point of skill over
  defense); no log here can show a difference that small, and the 2 below column adds none.
- Weapon skill against a lower target. The formulas take 0.04% a point off miss and dodge when
  skill passes the defense, while the level line below the character is 0.1% a point; nothing
  says how the two combine, so skill above base keeps moving nothing there.

## Where the range stops

- 5 above: the glancing chance follows its line to 5 above in the logs (58% of 416 swings for
  60%) and leaves it at 6 (62% of 541 for 70%, with ordinary hits still landing, which a 70%
  glancing chance and the misses, dodges and parries beside it would leave no room for). Spell
  miss reads 34% at 5 above for 39% and 42% at 6 for 50%. Nothing else covers 6 above.
- Level 10: the lowest level a character sims at, and the lowest dungeon boss is above it. From
  10 below the character every table has reached its floor, so a lower target changes nothing
  more; the logs reach 11 below.

## Sources

- Blizzard, WoW Classic, 30 May and 4 June 2019 (the "hit tables" bug thread, read from
  https://www.bluetracker.gg/wow/topic/us-en/185675-bug-hit-tables/): "Players have an 8% chance
  to miss a creature that is 3 levels above them", the first 1% of hit from talents or gear is
  ignored against a monster "more than 10 Defense Skill above", "Each additional level the target
  has over the player grants them 0.5% additional chance to dodge", a 14% parry at 3 above, and
  "Critical Strike chance is reduced by 1% per each additional level the target has over the
  player". They tested 5 below and 5 above and published neither.
- Blizzard, 5 October 2005, on spell hit against a creature: 96% at the caster's level, 83% at 3
  above, 72% at 4, 61% at 5 (read from a 2019 repost,
  https://us.forums.blizzard.com/en/wow/t/level-differences-in-pvp-are-not-calculating-correctly/296761).
- Community formulas (https://github.com/magey/classic-warrior/wiki/Attack-table, the
  warcraft.wiki.gg pages on miss, glancing blows, crushing blows and defense, and two open
  source server emulators). They agree from 0 to 3 above and disagree outside: miss past 3 above
  rises 1%, 2% or 3% a level, glancing is uncapped or capped at 40%, and below the attacker miss
  and dodge fall 0.5% or 0.2% a level. So the rules above come from logs wherever they differ.

## The logs

forever.warcraftlogs.com, zone 3000 (Dungeons), the first public reports of each dungeon's
listing on 23 November 2026: 305 reports across 27 dungeons, 4.58 million direct damage events
(1.17 million white swings, 1.01 million physical specials, 2.40 million spells). Three reports
were read twice by mistake, about 1% of the events.

- A player's level is its talent points plus 9, from each report's combatant info. It is a lower
  bound when points are unspent; level 60 is exact.
- A mob's level is Wowhead's Forever tooltip (`nether.wowhead.com/forever/tooltip/npc/<id>`),
  687 of 826 mobs; 650 are kept (level 10 or above, a range of at most 2 levels). The tables
  keep mobs the tooltip gives one level for unless they say otherwise.
- Events: `damage` events of players against enemies, with `hitType` 0 miss, 1 hit, 2 crit, 4
  block, 6 glancing, 7 dodge, 8 parry, 14 resist. Ticks are left out.
- Thirteen cells where one levelling player made more than 3,000 swings at one mob (one of them
  99.7% glancing blows on 11,783 swings) are left out.
- No player or character name was kept.

Levelling players (10 to 59) against mobs of one known level:

| Gap | White swings | Glancing | Glancing damage / hit | Special attack miss | Druid form swing miss | Caster spell miss |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 8 below | 946 | 0.9% | | 0.9% | 0.5% | none |
| 5 below | 3,308 | 2.3% | | 1.3% | 1.9% | 0.2% |
| 3 below | 5,721 | 4.0% | | 2.5% | 4.3% | 0.8% |
| 2 below | 6,984 | 3.8% | | 3.0% | 4.4% | 2.4% |
| 1 below | 7,846 | 5.9% | | 3.2% | 3.6% | 3.2% |
| 0 | 9,157 | 14.4% | 0.88 | 3.6% | 4.8% | 4.5% |
| 1 above | 6,700 | 24.0% | 0.93 | 4.1% | 4.9% | 4.3% |
| 2 above | 5,175 | 33.0% | 0.81 | 5.5% | 5.8% | 5.0% |
| 3 above | 3,203 | 41.8% | 0.62 | 8.7% | 8.3% | 16.9% |
| 4 above | 1,979 | 50.1% | 0.43 | 11.2% | 10.3% | 28.2% |
| 5 above | 416 | 58.2% | 0.35 at 4.5 | 14.7% | | 33.8% |
| 6 above | 541 | 61.9% | 0.12 to 0.17 | 18.9% | 14.6% | 42.2% |

The glancing damage column is each player's mean glancing blow over its mean ordinary hit on the
same mob, its median over players, from the third of the reports read with the amounts (the
rule: 0.95, 0.95, 0.85, 0.65, 0.45, 0.25).

Level 60 players, whose level is exact: 11.8% of 16,386 swings glance at their level, 20.3% of
6,420 at 1 above, 30.6% of 7,125 at 2 above, and 1.4% to 3.0% at 1 to 5 below. Dodge reads 3.5%
at their level and 1.5% at 6 below.

Druids in a form, whose attacks have no weapon skill to be behind on, against every mob below
them (83,000 swings, half levels are mobs with a two level range): miss 4.9% at half a level
below, 4.0% at 1.5, 3.2% at 3.5, 2.3% at 5.5, 0.9% at 7.5, 0.5% at 9.5 and none at 11.5; dodge
4.6%, 3.8%, 3.0%, 2.3%, 1.4%, 0.3% and none. A 0.2% a level rule would leave 3.5% at 7.5 below.

What they settle:

- Glancing is not capped at 40%. It keeps 10% a level to 5 above, and its damage follows the
  formula's ends.
- No swing glances off a lower target. The 1% to 4% left is weapons below their skill cap and
  players with unspent talent points.
- Below the attacker miss and dodge fall about 0.5% a level and are gone 10 levels down.
- Spell miss is Blizzard's table at 4 above.
- Miss past 3 above: at 4 above a Druid's form swings miss 10.3% (474 swings), a Paladin's 7.6%
  where its lower levels read 1.3% under the columns (394), a Shaman's 7.2% (221) and a Rogue's
  dual wield swings 24.9% for 26.7% (603). The classes together fit 9% better than 10% and rule
  out 12%.

Seen and left alone: the fight floors spell miss at 1%. Level 60 casters in these logs missed
0.13% of 2,374 casts on mobs 1 above them and 0.6% of 5,399 on mobs of their level, which a 1%
floor cannot give. Changing it would move every caster board, so it is its own question.

Report codes:

```
1DLQRC687z3ZrdPJ 1MmC7rTaWLhgF2bR 1d67pFkVrZQn2wqB 1dJQcXNTj8BnrkgK 1mF3gwpJXnjWd9Pq 
1tLCfdvQnH4mT9BY 23nYBkXTmh8KDAW1 26AjDVBGzP1rXLbK 2KpdCFTW8aAtVmPb 2MNXKFJ1d6Dayf4j 
2NYzkZRLG98Ad3XJ 2hX8yr9Zt1p3Abzv 2jtHJVfRyBvXnhMx 2mNyRawv7fGMDYVx 2pVHBkGvWDx7yrKq 
2v6C84gjpzXmRdqQ 36kf9Bd7MAwRPG8F 3Ba8q6cbpGAtjL4N 3GJ8NHbVxqLRPD92 3KCdD71J8bwWApac 
3bQCRkj1TA9PGB7g 3hBAqGwvrJD2VdZg 3nXRjDc4H2p6KvBJ 467YhgyBQvdGCtpa 46aq27WncjYHv3zM 
4JfTm67hHAkgFwKB 4bvGVM7f9zaFrPdN 4pgCK6rnQDc91Mhq 643GkWZzXYqKRcfA 674WwdkLvKFMRb3A 
6JZ3PN8KDF9gfG7M 6RNqGDz9rQfJZpMV 6YtRQVy7JH2LBwkx 6gDPKpbZAYN9XCxn 6yMrVb1ZqXRT2N9H 
712BqGKFfPAYp6dt 7FvxGYf63aNRQwVj 7TZmbghCpqjzvYxn 7Yt8ZRc6j2J9b3AP 7ZWFDG4XBPKqf8zR 
7k3dWxyMJ8pBAvqf 7mbkajTnAdPWBXCF 7vm9ZWDcdqyxH6VT 8AdWKRk61pcrhzN7 8N7YZFRxyqA1Pnc6 
8PWKDHTBgpmvc34j 8hVw1CZ7jPfKWmBn 8ra4CdwRzXvjT21D 91n7jc2zZxfkVdtX 96xXHWnZRLfgpDcF 
9FgHvwTfb8DZ7hPd 9KNaBPm7ywF31Z2h 9RpHrj4Ay8zmxvaB 9kGgJ3qN6RMbnhf1 AP4vBCgQj78qD6zW 
ARKQH1hywz7mV69r AXvy9JkawFZRgdNx AgKRnDwCxZyPVb9Q AhcynN8JvbkXVDj3 BHyhntaXJL4FD3mx 
BL6bWVdnthDQfzH3 Ba4bzVmX2nDrRN7k BgFkhMRrbpmtj8C2 Bk3vK7NwxWdDCgGX Bz2NMjHnY7PD6ykX 
C4HnNJV9rMDqvfkA C7KN6GpTnL2cgzqZ C8fF1Lvkxm6NrJ3a C8wLAnqJQbRHZNfh CBrqRVzfhATNdMjc 
CDpBXYzard74k1G2 CJmfAZk8FyhQbq3r CnZmWjckYfLqdFDJ Cr4Y2BKd9tmRQVZD CrmwA2LBW7k4T6Jp 
CtfJQxPvBKjTna9g CyB2t76n3YZHcVFj CyaXMxw2Nq6zgtFV DG4zL2tcATwyb8Q3 DTGwzYd9jvHVmQNb 
DqNzxPRhLZJfv9HV FG9xZBzNf4nWhaJY FGVZWbtNMnf2kmK3 FK2TQv9HJyhmLjBa FXYzWpG1VLHy6vcm 
Fn9XCRr6VYDGf84k FqTy4mXxnP8jvRKd FtnBkVJWmAq3f7gj FzRPjgcNfQZHthCa GBnj8F6zZP9CA7QH 
GDx2zcA64LfMKnvp GNc1DXLaZ3YKwbyd GThbgCWAkN74MBnd GXWK2YadVr7qDfm1 GdthYknQVMZxrKyF 
GfyxbaQhZvL8dT9H GhjAHCy6mvRVPFKJ GnKdPQkBza9c2Mhv HCPFmqwz23TVZ8hK HDwChqvd2bWKA3QP 
HJpXmBNg9fjv46Dr Hbhf2cp4P8v3k6yF HdhjpFmLNgxKa9TC HgkYc9bxMydfTD8q Hwy4GtazC6MNJ1vg 
J1rXBfnQN6t9mCz8 J4hMrG2LWXDwYKVN JKtg69hGbfcBxMqT JrcRx1F2PXAm84WG K7pnLG4fYBdRNxjD 
KCHpnWmcX6x4aQ81 KH8YAd72PWfMJN3V KMfvHBTFn6tACbhJ KTL7a4NXMC6yvVWg KcqHM1JZPC9y3gbj 
KftmrdQ7gFRX4yMY Kky7AVqvcP8tLxba KvXpw7g4WGnHdV1j KxrXLAQ2bfkc8dBF L6fp8aFmjBKZ3TWw 
LNvtnDxd1H6XfVGk LQNf1cbha46CFYqd LZGndkJYBtmQ4Tg6 MDApykqWP2hRcJjY MN9VDfYrbKTmwgkd 
MPWw1xA6tmD3fjJr MQFrhRPCHNwqLyJp MfQVdHCJG13KqzTN NPGa7w2gDztWHxcC NkM6KJa7b2Zdhc4Q 
Nqhwrcg1XnaFkYft P41prKYR7cvWzV96 PCXNbvw1dkB8Lx4H PR8dn7gT932ZbK6H PTgcXFyhr9JDxWKz 
PWLBTgqMYvDtQyVC PkVG6NDnLRFqhMzp PkmR9tq27AWXGfJK Pn4T7gD6JYd2NaLM PqYDXa98pFTKk1xJ 
PvXp62hV7yx1qrY9 PxXaDqRWdhQkf138 QL6PJyx3GvkHF1rp QL9CGcjVXJZDPABv QMLY8q2XNVry7bpj 
QwDMY8PBx2KgJby6 QwmphLYJXT7GazMd QzcPkmwRftXWpGgq QzjNR43MZh6XWDkP RkH6awfznjAhrtLd 
RpkxM8wrnGfJVPXZ T6qgwWC8VZjmJR3F TBz2LvdXaK6Cjtn3 TDGK2gWh1ry3Btaz Tjw2g6pHbWymaz7q 
TvwBmNMtQdJHrD2c VGyx9FR3fHBk2TDh VHTRfvkcWAt2hZxn VjhBnNT67dXGg4Fa W2n4aKqcXt7BMT1k 
W7MqPchwn3JTKpfF WFdcL8aB7P4hA1Mk WcCH7rnkQdaLZJ6j WhR29HAkMBDrGzap WhtFrapRdH6fbnc4 
WjDJXCHz8kwxTpQA X2LCqZRrVmgaJdvF X7jcKf164YFzdMy8 XMt4QRZJmNdjva3c XfBLnTDgpdYWkmzJ 
XpTKc72bB1DRgNzv Xtbyg4MJ9cpFYnWH Y29j4KZCAcvXy7Hx YLZ3CMf6KFWAV7rn YcVR36BKNkmgQLyb 
Yd41L2tAXbZ3Gqzw YdnywxfapJZ76Mt4 Z9J4PnX7VMvTfhrW ZVf9tDamcR1hk6AP ZnzydTGm4KXcfCkv 
a2vYjfVA1WFP3B76 a8pLcKTNvHQXMRY2 aJjwmQxBZAqc1zvd afFqv9HnMXrPAgxW akPAmT2FpcBQY8KN 
akfrAm74NYHVqKgP am6HpYG4nXgPw8cJ amX4Qv9krV8AMDcZ bQptMDdY1Z6r7GKA bY2j3zCagkAnXcD9 
bZYaDzrFgKVLC8tn bdZwRvQXNgCMzcaT bhQcRnC3wqLPHTxj br3Pp4zCm6hqZkg2 bvQzf18MmV9N6F3C 
c89Vvfhgk6Xjbw2M c8pNVWQY2Hn3Jhb9 cJgDKPT7ANv91bBy cW6gZwyJqaHd8CBD dBWxwrJn8L31kVGP 
dK6DcY8ftgqVvQR2 dTh4GAWLxwRm9V6k f42PyCAapJWZwxdF f7PmJcwZRtz3n2dq fMGk31BJWwv9NKnR 
fNB2DW4nqAzXwjCp fvmdF13tRnZ4X9y8 fw9yTpzNakdmgL4J fzpbd9XkAM3qHtYD g7A2Qf6RzqyW9bKx 
g7zX3qAx82dTvRWj gMzD2BC6aWNnmQfb gaVMRbjcmWCTnKq8 gcM93G6WpvH4bCwm gdhmfHb6pxRLXwFT 
ghjkdPvRxrBYNGq1 hm3NxtTAW1BYJpaV hqyNfmTQKRMLaj8z hwxz4y2NbJfpCtaj hzCTANdXK1y68YnW 
j1hPK3T4xvq2CJwG j3t41XcGdR2kr7xh jAVtCpkR7hc1zwmn jAzmxXBHqvDn91VF jZFXnJWmM3ar9DxB 
jhYXCmK4t9kQpwrH jr9PpAQZCd47vkay jvz81pMAWnr69hVf k2rzVWvydnTgxZ86 kC28G4wmJhPNLDA1 
kDPcRTgAYdbJm136 kKmz2aWphCHwjc7n kQqx8WpGzZFmD2a9 kXWbCf4gJvapDFyd m4dCL16tqnN2D3hW 
mHLWpCvFd1VYj8gB mTZXGt8k2PNYLrA6 mTkzdR2A1cx8hayg mZAY2HkFPcGKXDQL my78gaKBq9bDxMdZ 
n4Tz39pDCqwPH8WV n4q6fxmaLKB8wcXJ nKQ3V2rvTN4jYGPp p8thvQy92wMb7jcD pAv1CJfkr9DaYHZm 
pW1PZbJw6LAMzy4m pXaHvZRP29VAbMBD pydvC4YzqVMRgPAf q2bQzh3V8MXPvdtK q98JvNkWnKzyrwgM 
qHrXBCf9axZd8cPn qJKDbZWdPpjaLm2w qJjhQpNcr1tvVPfx qQZK1rFgLkDXbmyN qXZD3CbdpNGzYv1J 
qa6jrtCHcX8Fvwb9 qdLP9vcbjHKyZ6Mx qfay2cPntA6brzWg qvDpFLg8RPTfZA3m qyb3QNfcx2YJVmKH 
rDpJXPzM7xLjbq9Q rP3VYNn8mhAvaWpZ rf6mkbLyPZVn8z3g rfZt7pw6b2mVdBzC t2bxa3R1dAC9FJp8 
tFKXYdJgRMn2m3CW tJj8H7qb43hxXTcV tXk8HTWcJ1pMrNAV tgXxd8FZMB1PyLhR trxgPfKj3ALCFXDH 
tyxrVpvPfbngXQ4m vHjpcxGzbyMhAqJ6 vWNGydhJgbDPc9FQ vaXYqWCwxd7pKTg2 vfnQP3BCMkRmtGbX 
w1Jdq9Zv4738bGTF w1Mrf9TACycaXhvY wKDrPtmvqj1TbaAM wRKz13CtaZYfDMHg wZmPBCFXvjtYAG4x 
wdRgBVHMYcvX94fr wh29kQFm6crHZyTX wkqL2b49J3YrQVNv wtR76PQ8nxpANYkH x61fvm9RLMjXyDqB 
x8jwDJkF6Hqt37Cp xD4PmaJMrFkz1cH6 xHXB7zKtn3wkY2Lq xL9TgWpX1BhD3dG2 xLzNMKdfmW94yRQZ 
xYmak3JcyZQHMtVT xtzdapc1rLM6PVJm y4Krh32vqL1DHmjd yCAaZ2LP7wnJxp9F yD3PrJRazBHT4xpQ 
yL2Z3jpHdVqCarPF yPLkwGYbDW37QXhK yQ2DHjkGb9zqw1rK yR26CTxrwk7bjmVB yhBkdLV3aG8nzNDj 
zaM6vnQTd3K7kRY8 zbRNTQcwtZjPHChx zp6LV9HfmWc8BT3J zv8jV4gG16tCyhZq zvhWMGBk72c6ACJf

```
