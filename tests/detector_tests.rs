use soaim::detector::{default_words, Detector};

#[test]
fn total_occurrences_and_whole_word() {
    let d = Detector::new(["fuck", "shit"]).unwrap();
    assert_eq!(d.swear_count("fuck this shit"), 2);
    assert_eq!(d.swear_count("fuck fuck fuck"), 3);
    // whole-word: "fucking" not in list, must not match "fuck".
    assert_eq!(d.swear_count("fucking great"), 0);
}

#[test]
fn case_insensitive() {
    let d = Detector::new(["damn"]).unwrap();
    assert_eq!(d.swear_count("DAMN Damn damn"), 3);
}

#[test]
fn default_list_detects_common_profanity() {
    let d = Detector::new(default_words()).unwrap();
    assert!(d.contains_swear("this is fucking broken"));
    assert!(d.contains_swear("what the hell"));
    assert!(!d.contains_swear("a perfectly clean prompt"));
}
