use super::*;

fn character(point: u32) -> char {
    char::from_u32(point).expect("valid code point")
}

#[test]
fn removes_the_invisible_characters() {
    for point in [0x200B, 0x2060, 0xFEFF, 0x00AD, 0xFFFD] {
        let text = format!("we{}ll", character(point));
        assert_eq!(fix(&text, Tiers::default()), "well", "U+{point:04X}");
        assert_eq!(scan(&text, Tiers::default()).len(), 1, "U+{point:04X}");
    }
}

#[test]
fn replaces_typographic_punctuation() {
    let cases = [
        (0x00A0, "10 km"),
        (0x202F, "10 km"),
        (0x2009, "10 km"),
        (0x2013, "10-km"),
        (0x2014, "10-km"),
        (0x2212, "10-km"),
    ];
    for (point, expected) in cases {
        let text = format!("10{}km", character(point));
        assert_eq!(fix(&text, Tiers::default()), expected, "U+{point:04X}");
    }
    assert_eq!(
        fix(&format!("it{}s", character(0x2019)), Tiers::default()),
        "it's"
    );
    assert_eq!(
        fix(
            &format!("{}hi{}", character(0x201C), character(0x201D)),
            Tiers::default()
        ),
        "\"hi\""
    );
    assert_eq!(
        fix(&format!("wait{}", character(0x2026)), Tiers::default()),
        "wait..."
    );
}

#[test]
fn replaces_every_quote_variant() {
    for point in [0x2018, 0x2019, 0x201A, 0x201B] {
        let text = format!("a{}b", character(point));
        assert_eq!(fix(&text, Tiers::default()), "a'b", "U+{point:04X}");
    }
    for point in [0x201C, 0x201D, 0x201E] {
        let text = format!("a{}b", character(point));
        assert_eq!(fix(&text, Tiers::default()), "a\"b", "U+{point:04X}");
    }
}

#[test]
fn keeps_the_characters_that_carry_meaning() {
    for point in ALWAYS_ALLOWED {
        let text = format!("a{}b", character(point));
        assert_eq!(fix(&text, Tiers::default()), text, "U+{point:04X}");
        assert!(scan(&text, Tiers::default()).is_empty(), "U+{point:04X}");
    }
}

#[test]
fn leaves_other_scripts_and_emoji_alone() {
    let text = "Persian, emoji and CJK: sample text";
    assert_eq!(fix(text, Tiers::default()), text);
    assert!(scan(text, Tiers::default()).is_empty());
}

#[test]
fn reports_a_mojibake_pair_once() {
    let text = format!("{}{}", character(0x00E2), character(0x20AC));
    let findings = scan(&text, Tiers::default());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].tier, Tier::Mojibake);
    assert!(!findings[0].fixable);
}

#[test]
fn reports_a_c2_pair_once_rather_than_twice() {
    // The second half is U+00A0, which the punctuation tier bans on its own.
    let text = format!("{}{}", character(0x00C2), character(0x00A0));
    let findings = scan(&text, Tiers::default());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].tier, Tier::Mojibake);
}

#[test]
fn locates_a_finding_by_line_and_character_column() {
    let text = format!("first\nsecond {} here", character(0x2014));
    let findings = scan(&text, Tiers::default());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].line, 2);
    assert_eq!(findings[0].column, 8);
}

#[test]
fn counts_columns_in_characters_not_bytes() {
    let text = format!("aaa{}", character(0x2014));
    assert_eq!(scan(&text, Tiers::default())[0].column, 4);
}

#[test]
fn honours_the_selected_tiers() {
    let punctuation = format!("a{}b", character(0x2014));
    let invisible_only = Tiers {
        invisible: true,
        punctuation: false,
        mojibake: true,
    };
    assert!(scan(&punctuation, invisible_only).is_empty());
    assert_eq!(fix(&punctuation, invisible_only), punctuation);
    assert_eq!(scan(&punctuation, Tiers::default()).len(), 1);
}

#[test]
fn parses_a_comma_separated_tier_list() {
    let tiers = Tiers::from_list("invisible,mojibake").expect("valid list");
    assert!(tiers.invisible && tiers.mojibake && !tiers.punctuation);
    assert!(Tier::parse("punctuation").is_some());
}

#[test]
fn rejects_an_unknown_tier_name() {
    assert!(Tiers::from_list("invisible,typos").is_err());
}

#[test]
fn labels_every_tier() {
    assert_eq!(Tier::Invisible.label(), "invisible");
    assert_eq!(Tier::Punctuation.label(), "punctuation");
    assert_eq!(Tier::Mojibake.label(), "mojibake");
}

#[test]
fn displays_a_finding_as_a_code_point_and_a_name() {
    let findings = scan(&format!("a{}b", character(0x2014)), Tiers::default());
    assert_eq!(findings[0].to_string(), "U+2014 em dash");
}
